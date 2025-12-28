use std::io::{BufReader, Cursor};
use std::sync::{mpsc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use rodio::{Decoder, OutputStream, Sink, Source};

// Simple xorshift RNG for probability evaluation
#[derive(Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }
    
    fn next_f32(&mut self) -> f32 {
        // xorshift64
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        // Convert to 0.0..1.0
        (x as f32) / (u64::MAX as f32)
    }
}

use crate::model::song::Song;
use crate::model::track::TrackPlayback;
use crate::model::fx::Delay;
use crate::pattern::visual::Gate;

pub mod timing;
pub mod compile;
pub mod effects;

use compile::{visual_to_tokens_and_pitches, CompiledPattern, CompiledStep};
use timing::{base_step_period, gate_duration, pitch_semitones_to_speed, step_period_with_swing, velocity_to_gain};
use effects::{DelayEffect, parse_delay_time};

static PLAYING: AtomicBool = AtomicBool::new(false);

pub fn is_playing() -> bool {
    PLAYING.load(Ordering::SeqCst)
}

// ------ Live snapshot state for REPL ------
#[derive(Clone, Debug)]
pub struct LiveTrackSnapshot {
    pub name: String,
    pub token_index: usize,
    pub pattern: Vec<bool>,
}

#[derive(Clone, Debug)]
pub struct LiveSnapshot {
    pub tracks: Vec<LiveTrackSnapshot>,
    /// Global step position (0-indexed) for synchronized display
    pub global_step: usize,
}

fn live_state_cell() -> &'static Mutex<Option<LiveSnapshot>> {
    static CELL: OnceCell<Mutex<Option<LiveSnapshot>>> = OnceCell::new();
    CELL.get_or_init(|| Mutex::new(None))
}

pub fn snapshot_live_state() -> Option<LiveSnapshot> {
    live_state_cell().lock().ok().and_then(|g| g.clone())
}

#[derive(Clone)]
struct SequencerConfig {
    bpm: u32,
    swing: u8,
    repeat: bool,
    tracks: Vec<LoadedTrack>,
}

enum ControlMsg {
    Stop,
    Update(SequencerConfig),
}

fn transport() -> &'static Mutex<Option<mpsc::Sender<ControlMsg>>> {
    static CELL: OnceCell<Mutex<Option<mpsc::Sender<ControlMsg>>>> = OnceCell::new();
    CELL.get_or_init(|| Mutex::new(None))
}

/// Starts a background step sequencer using song bpm/steps and visual patterns.
/// Each hit triggers the track's sample; when `repeat` is on it runs indefinitely
/// until `stop()` is called.
pub fn play_song(song: &Song) -> Result<()> {
    // Stop any existing playback thread
    {
        let mut guard = transport().lock().unwrap();
        if let Some(tx) = guard.take() {
            let _ = tx.send(ControlMsg::Stop);
        }
    }

    let cfg = build_config(song);
    if cfg.tracks.is_empty() {
        return Err(anyhow!("no playable samples"));
    }

    let (tx, rx) = mpsc::channel::<ControlMsg>();
    std::thread::spawn(move || {
        let (_stream, stream_handle) = match OutputStream::try_default().context("opening audio output") {
            Ok(s) => s,
            Err(e) => {
                crate::console::error(format!("audio error: {}", e));
                return;
            }
        };
        let mut current = cfg;
        // Build per-track runtime state
        let mut rt = build_runtime(&current);
        let start = Instant::now();
        // Global step counter for synchronized display
        let mut global_step: usize = 0;
        let mut next_global_step_time = start;
        let mut end_deadline: Option<Instant> = if current.repeat {
            None
        } else {
            // Compute duration long enough for the longest pattern to complete once
            let max_secs = current
                .tracks
                .iter()
                .map(|t| (t.pattern.len() as f64 / t.div as f64) * (60.0 / current.bpm as f64))
                .fold(0.0, f64::max);
            Some(start + Duration::from_secs_f64(max_secs))
        };
        // mark playing on entry
        PLAYING.store(true, Ordering::SeqCst);
        'outer: loop {
            // Process control messages, rebuild runtime state on Update
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ControlMsg::Stop => {
                        for track in &mut rt {
                            stop_track_voices(track);
                        }
                        break 'outer;
                    }
                    ControlMsg::Update(new_cfg) => {
                        let now_u = Instant::now();
                        for track in &mut rt {
                            stop_track_voices(track);
                        }
                        rt = merge_runtime_preserving_phase(&current, &new_cfg, &rt, now_u, global_step, next_global_step_time);
                        current = new_cfg;
                        // reset end deadline based on new config
                        end_deadline = if current.repeat { None } else {
                            let max_secs = current
                                .tracks
                                .iter()
                                .map(|t| (t.pattern.len() as f64 / t.div as f64) * (60.0 / current.bpm as f64))
                                .fold(0.0, f64::max);
                            Some(now_u + Duration::from_secs_f64(max_secs))
                        };
                    }
                }
            }

            let now = Instant::now();
            if let Some(deadline) = end_deadline {
                if now >= deadline { break 'outer; }
            }

            // Fire any due tokens per track; advance their schedules
            for tr in &mut rt {
                service_track_voices(tr, now);
                process_pending_ratchets(&stream_handle, tr, now);
                while now >= tr.next_time {
                    if !tr.steps.is_empty() {
                        let idx = tr.token_index % tr.steps.len();
                        // Clone the step to avoid borrow issues
                        let step = tr.steps[idx].clone();
                        
                        if !step.events.is_empty() && !tr.muted {
                            if should_stop_current(tr.playback) {
                                stop_track_voices(tr);
                            }
                            
                            // Spawn a voice for each event in the step (chord polyphony)
                            for event in &step.events {
                                // Evaluate probability - skip if random exceeds threshold
                                if let Some(prob) = event.probability {
                                    let roll = tr.rng.next_f32();
                                    if roll > prob {
                                        continue; // Skip this event
                                    }
                                }
                                
                                let ratchet_count = event.ratchet.unwrap_or(1).max(1);
                                let sub_step_duration = tr.base_period / ratchet_count;
                                
                                for ratchet_idx in 0..ratchet_count {
                                    if ratchet_idx == 0 {
                                        // First hit: trigger immediately
                                        trigger_voice(
                                            &stream_handle,
                                            tr,
                                            event.pitch,
                                            event.velocity,
                                            event.gate,
                                            step.hold_steps,
                                            current.bpm,
                                        );
                                    } else {
                                        // Schedule subsequent ratchet hits
                                        let trigger_at = Instant::now() + sub_step_duration * ratchet_idx;
                                        tr.pending_ratchets.push(PendingRatchet {
                                            trigger_at,
                                            data: tr.data.clone(),
                                            pitch: event.pitch,
                                            velocity: event.velocity,
                                            gain: tr.gain,
                                            gate: event.gate,
                                            delay: tr.delay.clone(),
                                            base_period: sub_step_duration,
                                            playback: tr.playback,
                                            bpm: current.bpm,
                                        });
                                    }
                                }
                            }
                        }
                        tr.token_index = (tr.token_index + 1) % tr.steps.len();
                    }
                    // Advance by swing-adjusted step duration
                    let step_period = step_period_with_swing(
                        current.bpm,
                        tr.div,
                        current.swing,
                        tr.token_index.saturating_sub(1),
                    );
                    tr.next_time += step_period;
                }
            }
            let after_triggers = Instant::now();
            for tr in &mut rt {
                service_track_voices(tr, after_triggers);
                process_pending_ratchets(&stream_handle, tr, after_triggers);
            }

            // Advance global step counter based on time
            let global_step_period = base_step_period(current.bpm, 4); // 16th notes (div=4)
            while after_triggers >= next_global_step_time {
                global_step += 1;
                next_global_step_time += global_step_period;
            }

            // Update live snapshot for REPL display
            update_live_snapshot(&current, &rt, global_step.saturating_sub(1));

            // Sleep until the next nearest event to reduce CPU
            let next_step_due = rt.iter().map(|t| t.next_time).min();
            let next_voice_due = earliest_voice_deadline(&rt);
            let next_due = match (next_step_due, next_voice_due) {
                (Some(step), Some(voice)) => if step <= voice { step } else { voice },
                (Some(step), None) => step,
                (None, Some(voice)) => voice,
                (None, None) => now + Duration::from_millis(10),
            };
            let wait = if next_due > now { next_due - now } else { Duration::from_millis(1) };
            // Limit max wait to be responsive to updates
            let wait = wait.min(Duration::from_millis(25));
            std::thread::sleep(wait);
        }
        // ensure flag reset on thread exit
        for tr in &mut rt {
            stop_track_voices(tr);
        }
        PLAYING.store(false, Ordering::SeqCst);
        // Clear live snapshot on exit
        if let Ok(mut guard) = live_state_cell().lock() { *guard = None; }
    });

    // Store sender so we can stop
    let mut guard = transport().lock().unwrap();
    *guard = Some(tx);
    // Mark playing true once thread is spawned
    PLAYING.store(true, Ordering::SeqCst);
    Ok(())
}

pub fn stop() {
    let mut guard = transport().lock().unwrap();
    if let Some(tx) = guard.take() {
        let _ = tx.send(ControlMsg::Stop);
    }
    PLAYING.store(false, Ordering::SeqCst);
}

/// Preview a sample by playing it once (without affecting transport).
pub fn preview_sample(path: &str) -> Result<()> {
    use rodio::{Decoder, OutputStream, Sink};
    use std::fs::File;
    use std::io::BufReader;
    
    let file = File::open(path)
        .map_err(|e| anyhow::anyhow!("cannot open sample: {}", e))?;
    let reader = BufReader::new(file);
    let source = Decoder::new(reader)
        .map_err(|e| anyhow::anyhow!("cannot decode sample: {}", e))?;
    
    // Spawn a thread to play the sample (fire and forget)
    std::thread::spawn(move || {
        if let Ok((_stream, handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&handle) {
                sink.append(source);
                sink.sleep_until_end();
            }
        }
    });
    
    Ok(())
}

fn db_to_amplitude(db: f32) -> f32 {
    (10.0_f32).powf(db / 20.0)
}

#[derive(Clone)]
struct LoadedTrack {
    name: String,
    data: Vec<u8>,
    gain: f32,
    steps: Vec<CompiledStep>,
    pattern: Vec<bool>, // kept for live snapshot display
    delay: Delay,
    div: u32,
    muted: bool,
    playback: TrackPlayback,
}

// moved to audio::compile

// helpers moved to audio::{compile,timing}

pub fn reload_song(song: &Song) {
    let cfg = build_config(song);
    if cfg.tracks.is_empty() { return; }
    if let Some(tx) = transport().lock().unwrap().as_ref().cloned() {
        let _ = tx.send(ControlMsg::Update(cfg));
    }
}

fn build_config(song: &Song) -> SequencerConfig {
    let mut tracks = Vec::new();
    // If any track is solo, mute all non-solo tracks regardless of their mute flag.
    let any_solo = song.tracks.iter().any(|t| t.solo);
    for t in &song.tracks {
        let Some(path) = &t.sample else { continue; };
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                crate::console::warn(format!("warning: skipping track '{}': {}", t.name, e));
                continue;
            }
        };
        let compiled = match t.active_pattern() {
            Some(crate::model::pattern::Pattern::Visual(s)) => visual_to_tokens_and_pitches(s),
            None => CompiledPattern::empty(),
        };
        let muted = if any_solo { !t.solo } else { t.mute };
        tracks.push(LoadedTrack {
            name: t.name.clone(),
            data: bytes,
            gain: db_to_amplitude(t.gain_db),
            steps: compiled.steps,
            pattern: compiled.triggers,
            delay: t.delay.clone(),
            div: t.div.max(1),
            muted,
            playback: t.playback,
        });
    }
    SequencerConfig { bpm: song.bpm, swing: song.swing, repeat: song.repeat_on(), tracks }
}

#[cfg(test)]
fn is_muted(any_solo: bool, mute: bool, solo: bool) -> bool {
    if any_solo { !solo } else { mute }
}

struct TrackRuntime {
    data: Vec<u8>,
    gain: f32,
    steps: Vec<CompiledStep>,
    pattern: Vec<bool>, // kept for live snapshot display
    delay: Delay,
    base_period: Duration,
    next_time: Instant,
    token_index: usize,
    muted: bool,
    div: u32,
    playback: TrackPlayback,
    voices: Vec<VoiceHandle>,
    pending_ratchets: Vec<PendingRatchet>,
    rng: SimpleRng, // For probability evaluation
}

struct VoiceHandle {
    sink: Sink,
    stop_at: Option<Instant>,
}

/// Pending ratchet sub-hit to be triggered at a specific time.
#[derive(Clone)]
struct PendingRatchet {
    trigger_at: Instant,
    data: Vec<u8>,
    pitch: i32,
    velocity: u8,
    gain: f32,
    gate: Option<Gate>,
    delay: Delay,
    base_period: Duration,
    playback: TrackPlayback,
    bpm: u32,
}

fn service_track_voices(track: &mut TrackRuntime, now: Instant) {
    for voice in track.voices.iter_mut() {
        if let Some(deadline) = voice.stop_at {
            if now >= deadline {
                voice.sink.stop();
                voice.stop_at = None;
            }
        }
    }
    track.voices.retain(|voice| !voice.sink.empty());
}

/// Trigger a single voice with the given parameters.
fn trigger_voice(
    stream_handle: &rodio::OutputStreamHandle,
    tr: &mut TrackRuntime,
    pitch: i32,
    velocity: u8,
    gate: Option<Gate>,
    hold_steps: usize,
    bpm: u32,
) {
    let cursor = Cursor::new(tr.data.clone());
    let reader = BufReader::new(cursor);
    let decoded = match Decoder::new(reader) {
        Ok(s) => s,
        Err(e) => {
            crate::console::error(format!("audio decode error: {}", e));
            return;
        }
    };
    
    let speed = pitch_semitones_to_speed(pitch);
    let vel_gain = velocity_to_gain(velocity);
    
    // Build source with pitch and gain
    let base_source = decoded
        .speed(speed)
        .amplify(tr.gain * vel_gain)
        .convert_samples::<f32>();
    
    // Apply delay if enabled
    let final_source: Box<dyn Source<Item = f32> + Send> = if tr.delay.on {
        let delay_time = parse_delay_time(&tr.delay.time, bpm);
        Box::new(DelayEffect::new(
            base_source,
            delay_time,
            tr.delay.feedback,
            tr.delay.mix,
        ))
    } else {
        Box::new(base_source)
    };
    
    match Sink::try_new(stream_handle) {
        Ok(sink) => {
            let start_instant = Instant::now();
            sink.append(final_source);
            sink.play();
            let stop_at = gate_stop_deadline(
                tr.playback,
                start_instant,
                tr.base_period,
                hold_steps.max(1),
                gate,
            );
            tr.voices.push(VoiceHandle { sink, stop_at });
        }
        Err(e) => crate::console::error(format!("audio error: {}", e)),
    }
}

/// Process pending ratchet sub-hits that are due.
fn process_pending_ratchets(
    stream_handle: &rodio::OutputStreamHandle,
    track: &mut TrackRuntime,
    now: Instant,
) {
    let mut triggered = Vec::new();
    for (i, ratchet) in track.pending_ratchets.iter().enumerate() {
        if now >= ratchet.trigger_at {
            triggered.push(i);
        }
    }
    
    // Trigger in reverse order to avoid index issues when removing
    for i in triggered.into_iter().rev() {
        let ratchet = track.pending_ratchets.remove(i);
        
        let cursor = Cursor::new(ratchet.data);
        let reader = BufReader::new(cursor);
        let decoded = match Decoder::new(reader) {
            Ok(s) => s,
            Err(e) => {
                crate::console::error(format!("audio decode error: {}", e));
                continue;
            }
        };
        
        let speed = pitch_semitones_to_speed(ratchet.pitch);
        let vel_gain = velocity_to_gain(ratchet.velocity);
        
        let base_source = decoded
            .speed(speed)
            .amplify(ratchet.gain * vel_gain)
            .convert_samples::<f32>();
        
        let final_source: Box<dyn Source<Item = f32> + Send> = if ratchet.delay.on {
            let delay_time = parse_delay_time(&ratchet.delay.time, ratchet.bpm);
            Box::new(DelayEffect::new(
                base_source,
                delay_time,
                ratchet.delay.feedback,
                ratchet.delay.mix,
            ))
        } else {
            Box::new(base_source)
        };
        
        if let Ok(sink) = Sink::try_new(stream_handle) {
            let start_instant = Instant::now();
            sink.append(final_source);
            sink.play();
            let stop_at = gate_stop_deadline(
                ratchet.playback,
                start_instant,
                ratchet.base_period,
                1,
                ratchet.gate,
            );
            track.voices.push(VoiceHandle { sink, stop_at });
        }
    }
}

fn stop_track_voices(track: &mut TrackRuntime) {
    for voice in track.voices.drain(..) {
        voice.sink.stop();
    }
}

fn should_stop_current(playback: TrackPlayback) -> bool {
    matches!(playback, TrackPlayback::Mono)
}

fn gate_stop_deadline(
    playback: TrackPlayback,
    start: Instant,
    base_step: Duration,
    hold_steps: usize,
    gate: Option<Gate>,
) -> Option<Instant> {
    if !matches!(playback, TrackPlayback::Gate) {
        return None;
    }
    let steps = hold_steps.max(1);
    let first = gate
        .map(|g| gate_duration(base_step, g))
        .unwrap_or(base_step);
    let remaining_steps = steps.saturating_sub(1) as f64;
    let rest = if remaining_steps > 0.0 {
        Duration::from_secs_f64(base_step.as_secs_f64() * remaining_steps)
    } else {
        Duration::from_secs(0)
    };
    Some(start + first + rest)
}

fn earliest_voice_deadline(tracks: &[TrackRuntime]) -> Option<Instant> {
    tracks
        .iter()
        .flat_map(|track| track.voices.iter().filter_map(|voice| voice.stop_at))
        .min()
}

fn build_runtime(cfg: &SequencerConfig) -> Vec<TrackRuntime> {
    let now = Instant::now();
    cfg.tracks
        .iter()
        .enumerate()
        .map(|(i, t)| TrackRuntime {
            data: t.data.clone(),
            gain: t.gain,
            steps: t.steps.clone(),
            pattern: t.pattern.clone(),
            delay: t.delay.clone(),
            base_period: base_step_period(cfg.bpm, t.div),
            next_time: now,
            token_index: 0,
            muted: t.muted,
            div: t.div,
            playback: t.playback,
            voices: Vec::new(),
            pending_ratchets: Vec::new(),
            rng: SimpleRng::new((i as u64 + 1) * 12345), // Seed based on track index
        })
        .collect()
}

fn merge_runtime_preserving_phase(
    old_cfg: &SequencerConfig,
    new_cfg: &SequencerConfig,
    old_rt: &[TrackRuntime],
    now: Instant,
    _global_step: usize,
    next_global_step_time: Instant,
) -> Vec<TrackRuntime> {
    use std::collections::HashMap;
    // We match by position/name from the previous config; if ordering changes,
    // we fall back to starting the new track from next boundary.
    // Build name->runtime index from old_cfg ordering
    let mut name_to_rt: HashMap<&str, &TrackRuntime> = HashMap::new();
    for (i, t) in old_cfg.tracks.iter().enumerate() {
        if let Some(rt) = old_rt.get(i) {
            name_to_rt.insert(t.name.as_str(), rt);
        }
    }

    // Pass 1: preserve phase for existing tracks; also pick the master (longest loop in beats).
    struct MasterCandidate {
        name: String,
        pattern_len: usize,
        div: u32,
        token_index: usize,
        next_time: Instant,
        beats_num: u64,
        beats_den: u64,
    }

    let mut preserved: HashMap<String, TrackRuntime> = HashMap::new();
    let mut master: Option<MasterCandidate> = None;

    for t in &new_cfg.tracks {
        let Some(old_rt) = name_to_rt.get(t.name.as_str()) else { continue };

        let new_period = base_step_period(new_cfg.bpm, t.div);
        // Compute remaining time in old schedule
        let old_period = old_rt.base_period;
        let remaining_old = time_until_next(now, old_rt.next_time, old_period);
        let new_remaining = if old_period.as_nanos() > 0 {
            let scale = new_period.as_secs_f64() / old_period.as_secs_f64();
            Duration::from_secs_f64((remaining_old.as_secs_f64() * scale).max(0.0))
        } else {
            new_period
        };
        let new_next = now + new_remaining;
        let new_token_index = if t.pattern.is_empty() { 0 } else { old_rt.token_index % t.pattern.len() };

        let rt = TrackRuntime {
            data: t.data.clone(),
            gain: t.gain,
            steps: t.steps.clone(),
            pattern: t.pattern.clone(),
            delay: t.delay.clone(),
            base_period: new_period,
            next_time: new_next,
            token_index: new_token_index,
            muted: t.muted,
            div: t.div,
            playback: t.playback,
            voices: Vec::new(),
            pending_ratchets: Vec::new(),
            rng: old_rt.rng.clone(), // Preserve RNG state
        };

        // Master selection: longest loop in beats (pattern_len / div)
        let pattern_len = t.pattern.len();
        if pattern_len > 0 {
            let beats_num = pattern_len as u64;
            let beats_den = t.div.max(1) as u64;
            let replace = match &master {
                None => true,
                Some(m) => (beats_num as u128) * (m.beats_den as u128) > (m.beats_num as u128) * (beats_den as u128),
            };
            if replace {
                master = Some(MasterCandidate {
                    name: t.name.clone(),
                    pattern_len,
                    div: t.div.max(1),
                    token_index: rt.token_index,
                    next_time: rt.next_time,
                    beats_num,
                    beats_den,
                });
            }
        }

        preserved.insert(t.name.clone(), rt);
    }

    let master_downbeat_time = if let Some(m) = &master {
        next_loop_downbeat_time(
            now,
            m.next_time,
            m.token_index,
            m.pattern_len,
            new_cfg.bpm,
            m.div,
            new_cfg.swing,
        )
    } else {
        next_global_step_time
    };

    // Pass 2: build output in new_cfg order; new tracks start fresh at the master downbeat.
    let mut out: Vec<TrackRuntime> = Vec::with_capacity(new_cfg.tracks.len());
    for (i, t) in new_cfg.tracks.iter().enumerate() {
        if let Some(rt) = preserved.remove(&t.name) {
            out.push(rt);
            continue;
        }
        let new_period = base_step_period(new_cfg.bpm, t.div);
        out.push(TrackRuntime {
            data: t.data.clone(),
            gain: t.gain,
            steps: t.steps.clone(),
            pattern: t.pattern.clone(),
            delay: t.delay.clone(),
            base_period: new_period,
            next_time: master_downbeat_time,
            token_index: 0,
            muted: t.muted,
            div: t.div,
            playback: t.playback,
            voices: Vec::new(),
            pending_ratchets: Vec::new(),
            rng: SimpleRng::new((i as u64 + 1) * 12345),
        });
    }
    out
}

fn time_until_next(now: Instant, next_time: Instant, period: Duration) -> Duration {
    if period.is_zero() {
        return Duration::from_millis(0);
    }
    if next_time > now {
        next_time - now
    } else {
        let p = period.as_secs_f64();
        let late = (now - next_time).as_secs_f64();
        let rem = p - (late % p);
        if rem == p { Duration::from_millis(0) } else { Duration::from_secs_f64(rem) }
    }
}

fn next_loop_downbeat_time(
    _now: Instant,
    next_time: Instant,
    token_index: usize,
    pattern_len: usize,
    bpm: u32,
    div: u32,
    swing: u8,
) -> Instant {
    if pattern_len == 0 {
        return next_time;
    }
    let token_index = token_index % pattern_len;
    if token_index == 0 {
        return next_time;
    }
    let mut t = next_time;
    for step_idx in token_index..pattern_len {
        t += step_period_with_swing(bpm, div, swing, step_idx);
    }
    t
}

// --- Swing helpers (pure, testable) ---
// moved to audio::timing

fn update_live_snapshot(cfg: &SequencerConfig, rt: &[TrackRuntime], global_step: usize) {
    let mut tracks = Vec::with_capacity(cfg.tracks.len());
    for (i, t) in cfg.tracks.iter().enumerate() {
        if let Some(r) = rt.get(i) {
            // token_index points to the NEXT step to play, so subtract 1 to show current
            let len = r.pattern.len();
            let display_index = if len == 0 {
                0
            } else if r.token_index == 0 {
                len - 1  // wrap around
            } else {
                (r.token_index - 1) % len
            };
            tracks.push(LiveTrackSnapshot {
                name: t.name.clone(),
                token_index: display_index,
                pattern: r.pattern.clone(),
            });
        }
    }
    let snap = LiveSnapshot { tracks, global_step };
    if let Ok(mut guard) = live_state_cell().lock() {
        *guard = Some(snap);
    }
}

#[cfg(test)]
mod build_config_tests {
    use super::is_muted;

    #[test]
    fn solo_overrides_mute_logic() {
        // No solos: respect mute flag
        assert_eq!(is_muted(false, false, false), false);
        assert_eq!(is_muted(false, true, false), true);
        // With any solo: only solo tracks sound
        assert_eq!(is_muted(true, false, false), true); // non-solo muted
        assert_eq!(is_muted(true, true, false), true);  // non-solo muted even if muted already
        assert_eq!(is_muted(true, true, true), false);  // solo plays
        assert_eq!(is_muted(true, false, true), false); // solo plays
    }
}

#[cfg(test)]
mod playback_mode_tests {
    use super::{build_config, gate_stop_deadline, should_stop_current};
    use crate::model::{
        pattern::Pattern,
        song::Song,
        track::{Track, TrackPlayback},
    };
    use crate::pattern::visual::Gate;
    use std::time::{Duration, Instant};

    #[test]
    fn mono_mode_stops_current_voice() {
        assert!(should_stop_current(TrackPlayback::Mono));
    }

    #[test]
    fn one_shot_mode_allows_layered_voices() {
        assert!(!should_stop_current(TrackPlayback::OneShot));
    }

    #[test]
    fn gate_mode_schedules_stop_for_step_length() {
        let now = Instant::now();
        let period = Duration::from_millis(120);
        let stop = gate_stop_deadline(TrackPlayback::Gate, now, period, 1, None)
            .expect("gate should schedule stop");
        assert!(stop >= now + period);
    }

    #[test]
    fn gate_mode_respects_ties() {
        let now = Instant::now();
        let period = Duration::from_millis(100);
        let stop = gate_stop_deadline(TrackPlayback::Gate, now, period, 4, None)
            .expect("gate should schedule stop");
        let expected = Duration::from_secs_f64(period.as_secs_f64() * 4.0);
        assert!(stop >= now + expected);
    }

    #[test]
    fn gate_mode_respects_gate_fraction() {
        let now = Instant::now();
        let period = Duration::from_millis(200);
        let gate = Gate::Percent(50.0);
        let stop = gate_stop_deadline(TrackPlayback::Gate, now, period, 3, Some(gate))
            .expect("gate should schedule stop");
        let first = super::gate_duration(period, gate);
        let rest = Duration::from_secs_f64(period.as_secs_f64() * 2.0);
        assert!(stop >= now + first + rest);
    }

    #[test]
    fn non_gate_modes_do_not_schedule_stop() {
        let now = Instant::now();
        let period = Duration::from_millis(120);
        assert!(gate_stop_deadline(TrackPlayback::OneShot, now, period, 1, None).is_none());
        assert!(gate_stop_deadline(TrackPlayback::Mono, now, period, 1, None).is_none());
    }

    #[test]
    fn build_config_preserves_visual_sustain_steps() {
        let mut song = Song::default();
        let mut track = Track::new("Synth");
        track.sample = Some("samples/synth/C4.wav".to_string());
        track.pattern = Some(Pattern::visual("x_______........"));
        track.playback = TrackPlayback::Gate;
        track.div = 4;
        song.tracks.push(track);

        let cfg = build_config(&song);
        assert_eq!(cfg.tracks.len(), 1);
        // Check sustain via the new steps structure
        let sustain = cfg.tracks[0].steps.get(0).map(|s| s.hold_steps);
        assert_eq!(sustain, Some(8));
    }
}

#[cfg(test)]
mod merge_runtime_phase_tests {
    use super::*;
    use crate::model::fx::Delay;
    use crate::model::track::TrackPlayback;
    use std::time::Duration;

    fn lt(
        name: &str,
        pattern_len: usize,
        div: u32,
    ) -> LoadedTrack {
        LoadedTrack {
            name: name.to_string(),
            data: Vec::new(),
            gain: 1.0,
            steps: Vec::new(),
            pattern: vec![false; pattern_len],
            delay: Delay::default(),
            div: div.max(1),
            muted: false,
            playback: TrackPlayback::Gate,
        }
    }

    fn rt(
        bpm: u32,
        div: u32,
        pattern_len: usize,
        token_index: usize,
        next_time: Instant,
    ) -> TrackRuntime {
        TrackRuntime {
            data: Vec::new(),
            gain: 1.0,
            steps: Vec::new(),
            pattern: vec![false; pattern_len],
            delay: Delay::default(),
            base_period: base_step_period(bpm, div),
            next_time,
            token_index,
            muted: false,
            div,
            playback: TrackPlayback::Gate,
            voices: Vec::new(),
            pending_ratchets: Vec::new(),
            rng: SimpleRng::new(12345),
        }
    }

    #[test]
    fn new_track_waits_until_next_downbeat_of_longest_loop_in_beats() {
        let now = Instant::now();
        // Track A: longer token pattern, but shorter in beats (16/4 = 4 beats)
        let a = lt("A", 16, 4);
        // Track B: shorter token pattern, but longer in beats (8/1 = 8 beats) => master
        let b = lt("B", 8, 1);
        let c = lt("C", 4, 4); // new track

        let old_cfg = SequencerConfig { bpm: 60, swing: 0, repeat: true, tracks: vec![a.clone(), b.clone()] };
        let new_cfg = SequencerConfig { bpm: 60, swing: 0, repeat: true, tracks: vec![a.clone(), b.clone(), c.clone()] };

        // Existing runtime state: A is first, but B is the longest loop in beats.
        let old_rt = vec![
            rt(old_cfg.bpm, a.div, a.pattern.len(), 0, now + Duration::from_millis(100)),
            rt(old_cfg.bpm, b.div, b.pattern.len(), 3, now + Duration::from_millis(200)),
        ];

        let merged = merge_runtime_preserving_phase(&old_cfg, &new_cfg, &old_rt, now, 0, now);
        assert_eq!(merged.len(), 3);

        // Existing tracks keep their phase.
        assert_eq!(merged[0].token_index, 0);
        assert_eq!(merged[0].next_time, now + Duration::from_millis(100));
        assert_eq!(merged[1].token_index, 3);
        assert_eq!(merged[1].next_time, now + Duration::from_millis(200));

        // New track starts fresh, but waits until next downbeat of master (track B).
        assert_eq!(merged[2].token_index, 0);
        // Track B token_index=3, len=8 => downbeat at next_time + (8-3) * 1s = +5s
        let expected = Duration::from_millis(200) + Duration::from_secs(5);
        let actual = merged[2].next_time.duration_since(now);
        assert_eq!(actual, expected);
    }
}
