use std::io::{BufReader, Cursor};
use std::sync::{mpsc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use rodio::{Decoder, OutputStream, Sink, Source};

use crate::model::song::Song;
use crate::model::track::TrackPlayback;
use crate::pattern::visual::Gate;

pub mod timing;
pub mod compile;

use compile::{visual_to_tokens_and_pitches, CompiledPattern};
use timing::{base_step_period, gate_duration, pitch_semitones_to_speed, step_period_with_swing};

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

    let names: Vec<String> = cfg.tracks.iter().map(|l| l.name.clone()).collect();
    let (tx, rx) = mpsc::channel::<ControlMsg>();
    std::thread::spawn(move || {
        let (_stream, stream_handle) = match OutputStream::try_default().context("opening audio output") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("audio error: {}", e);
                return;
            }
        };
        println!("sequencing: {}", names.join(", "));
        let mut current = cfg;
        // Build per-track runtime state
        let mut rt = build_runtime(&current);
        let start = Instant::now();
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
                        rt = merge_runtime_preserving_phase(&current, &new_cfg, &rt, now_u);
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
                while now >= tr.next_time {
                    if !tr.pattern.is_empty() {
                        let idx = tr.token_index % tr.pattern.len();
                        let hit = tr.pattern[idx];
                        if hit && !tr.muted {
                            if should_stop_current(tr.playback) {
                                stop_track_voices(tr);
                            }
                            let cursor = Cursor::new(tr.data.clone());
                            let reader = BufReader::new(cursor);
                            let source = match Decoder::new(reader) {
                                Ok(s) => {
                                    let speed = tr
                                        .pitches
                                        .get(idx)
                                        .and_then(|p| *p)
                                        .map(pitch_semitones_to_speed)
                                        .unwrap_or(1.0);
                                    s.speed(speed).amplify(tr.gain)
                                },
                                Err(e) => { eprintln!("audio decode error: {}", e); break; }
                            };
                            let hold_steps = tr
                                .gate_steps
                                .get(idx)
                                .copied()
                                .unwrap_or(1)
                                .max(1);
                            let gate = tr
                                .gates
                                .get(idx)
                                .copied()
                                .flatten();
                            match Sink::try_new(&stream_handle) {
                                Ok(sink) => {
                                    let start_instant = Instant::now();
                                    sink.append(source);
                                    sink.play();
                                    let stop_at = gate_stop_deadline(
                                        tr.playback,
                                        start_instant,
                                        tr.base_period,
                                        hold_steps,
                                        gate,
                                    );
                                    tr.voices.push(VoiceHandle { sink, stop_at });
                                }
                                Err(e) => eprintln!("audio error: {}", e),
                            }
                        }
                        tr.token_index = (tr.token_index + 1) % tr.pattern.len();
                    }
                    // Advance by swing-adjusted step duration
                    let step = step_period_with_swing(
                        current.bpm,
                        tr.div,
                        current.swing,
                        tr.token_index.saturating_sub(1),
                    );
                    tr.next_time += step;
                }
            }
            let after_triggers = Instant::now();
            for tr in &mut rt {
                service_track_voices(tr, after_triggers);
            }

            // Update live snapshot for REPL display
            update_live_snapshot(&current, &rt);

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

fn db_to_amplitude(db: f32) -> f32 {
    (10.0_f32).powf(db / 20.0)
}

#[derive(Clone)]
struct LoadedTrack {
    name: String,
    data: Vec<u8>,
    gain: f32,
    pattern: Vec<bool>,
    pitches: Vec<Option<i32>>, // semitone offsets per step
    gate_steps: Vec<usize>,
    gates: Vec<Option<Gate>>,
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
                eprintln!("warning: skipping track '{}': {}", t.name, e);
                continue;
            }
        };
        let compiled = match &t.pattern {
            Some(crate::model::pattern::Pattern::Visual(s)) => visual_to_tokens_and_pitches(s),
            None => CompiledPattern::empty(),
        };
        let muted = if any_solo { !t.solo } else { t.mute };
        tracks.push(LoadedTrack {
            name: t.name.clone(),
            data: bytes,
            gain: db_to_amplitude(t.gain_db),
            pattern: compiled.triggers,
            pitches: compiled.pitches,
            gate_steps: compiled.hold_steps,
            gates: compiled.gates,
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
    pattern: Vec<bool>,
    pitches: Vec<Option<i32>>,
    gate_steps: Vec<usize>,
    gates: Vec<Option<Gate>>,
    base_period: Duration,
    next_time: Instant,
    token_index: usize,
    muted: bool,
    div: u32,
    playback: TrackPlayback,
    voices: Vec<VoiceHandle>,
}

struct VoiceHandle {
    sink: Sink,
    stop_at: Option<Instant>,
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
        .map(|t| TrackRuntime {
            data: t.data.clone(),
            gain: t.gain,
            pattern: t.pattern.clone(),
            pitches: t.pitches.clone(),
            gate_steps: t.gate_steps.clone(),
            gates: t.gates.clone(),
            base_period: base_step_period(cfg.bpm, t.div),
            next_time: now,
            token_index: 0,
            muted: t.muted,
            div: t.div,
            playback: t.playback,
            voices: Vec::new(),
        })
        .collect()
}

fn merge_runtime_preserving_phase(
    old_cfg: &SequencerConfig,
    new_cfg: &SequencerConfig,
    old_rt: &[TrackRuntime],
    now: Instant,
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

    // Construct new runtime preserving phase when possible
    let mut out: Vec<TrackRuntime> = Vec::with_capacity(new_cfg.tracks.len());
    for t in &new_cfg.tracks {
        let new_period = base_step_period(new_cfg.bpm, t.div);
        if let Some(old_rt) = name_to_rt.get(t.name.as_str()) {
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
            out.push(TrackRuntime {
                data: t.data.clone(),
                gain: t.gain,
                pattern: t.pattern.clone(),
                pitches: t.pitches.clone(),
                gate_steps: t.gate_steps.clone(),
                gates: t.gates.clone(),
                base_period: new_period,
                next_time: new_next,
                token_index: new_token_index,
                muted: t.muted,
                div: t.div,
                playback: t.playback,
                voices: Vec::new(),
            });
        } else {
            // New track: schedule from next token boundary
            out.push(TrackRuntime {
                data: t.data.clone(),
                gain: t.gain,
                pattern: t.pattern.clone(),
                pitches: t.pitches.clone(),
                gate_steps: t.gate_steps.clone(),
                gates: t.gates.clone(),
                base_period: new_period,
                next_time: now + new_period,
                token_index: 0,
                muted: t.muted,
                div: t.div,
                playback: t.playback,
                voices: Vec::new(),
            });
        }
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

// --- Swing helpers (pure, testable) ---
// moved to audio::timing

fn update_live_snapshot(cfg: &SequencerConfig, rt: &[TrackRuntime]) {
    let mut tracks = Vec::with_capacity(cfg.tracks.len());
    for (i, t) in cfg.tracks.iter().enumerate() {
        if let Some(r) = rt.get(i) {
            tracks.push(LiveTrackSnapshot {
                name: t.name.clone(),
                token_index: if r.pattern.is_empty() { 0 } else { r.token_index % r.pattern.len() },
                pattern: r.pattern.clone(),
            });
        }
    }
    let snap = LiveSnapshot { tracks };
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
        let sustain = cfg.tracks[0].gate_steps.get(0).copied();
        assert_eq!(sustain, Some(8));
    }
}
