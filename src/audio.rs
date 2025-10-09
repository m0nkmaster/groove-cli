use std::io::{BufReader, Cursor};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use rodio::{Decoder, OutputStream, Sink, Source};

use crate::model::song::Song;

#[derive(Clone)]
struct SequencerConfig {
    bpm: u32,
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
        let mut voices: Vec<Sink> = Vec::new();
        'outer: loop {
            // Process control messages, rebuild runtime state on Update
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ControlMsg::Stop => {
                        for s in voices.drain(..) { s.stop(); }
                        break 'outer;
                    }
                    ControlMsg::Update(new_cfg) => {
                        let now_u = Instant::now();
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
                while now >= tr.next_time {
                    if !tr.pattern.is_empty() {
                        let hit = tr.pattern[tr.token_index % tr.pattern.len()];
                        if hit && !tr.muted {
                            let cursor = Cursor::new(tr.data.clone());
                            let reader = BufReader::new(cursor);
                            let source = match Decoder::new(reader) {
                                Ok(s) => s.amplify(tr.gain),
                                Err(e) => { eprintln!("audio decode error: {}", e); break; }
                            };
                            match Sink::try_new(&stream_handle) {
                                Ok(sink) => { sink.append(source); sink.play(); voices.push(sink); }
                                Err(e) => eprintln!("audio error: {}", e),
                            }
                        }
                        tr.token_index = (tr.token_index + 1) % tr.pattern.len();
                    }
                    tr.next_time += tr.period;
                }
            }

            // Clean up finished voices
            voices.retain(|s| !s.empty());

            // Sleep until the next nearest event to reduce CPU
            let next_due = rt.iter().map(|t| t.next_time).min().unwrap_or(now + Duration::from_millis(10));
            let wait = if next_due > now { next_due - now } else { Duration::from_millis(1) };
            // Limit max wait to be responsive to updates
            let wait = wait.min(Duration::from_millis(25));
            std::thread::sleep(wait);
        }
    });

    // Store sender so we can stop
    let mut guard = transport().lock().unwrap();
    *guard = Some(tx);
    Ok(())
}

pub fn stop() {
    let mut guard = transport().lock().unwrap();
    if let Some(tx) = guard.take() {
        let _ = tx.send(ControlMsg::Stop);
    }
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
    div: u32,
    muted: bool,
}

fn visual_to_tokens(s: &str) -> Vec<bool> {
    let tokens: Vec<bool> = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| matches!(c, 'x' | 'X' | '1' | '*'))
        .collect();
    tokens
}

pub fn reload_song(song: &Song) {
    let cfg = build_config(song);
    if cfg.tracks.is_empty() { return; }
    if let Some(tx) = transport().lock().unwrap().as_ref().cloned() {
        let _ = tx.send(ControlMsg::Update(cfg));
    }
}

fn build_config(song: &Song) -> SequencerConfig {
    let mut tracks = Vec::new();
    for t in &song.tracks {
        let Some(path) = &t.sample else { continue; };
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: skipping track '{}': {}", t.name, e);
                continue;
            }
        };
        let pattern = match &t.pattern {
            Some(crate::model::pattern::Pattern::Visual(s)) => visual_to_tokens(s),
            None => Vec::new(),
        };
        tracks.push(LoadedTrack {
            name: t.name.clone(),
            data: bytes,
            gain: db_to_amplitude(t.gain_db),
            pattern,
            div: t.div.max(1),
            muted: t.mute,
        });
    }
    SequencerConfig { bpm: song.bpm, repeat: song.repeat_on(), tracks }
}

struct TrackRuntime {
    data: Vec<u8>,
    gain: f32,
    pattern: Vec<bool>,
    period: Duration,
    next_time: Instant,
    token_index: usize,
    muted: bool,
}

fn build_runtime(cfg: &SequencerConfig) -> Vec<TrackRuntime> {
    let now = Instant::now();
    cfg.tracks
        .iter()
        .map(|t| TrackRuntime {
            data: t.data.clone(),
            gain: t.gain,
            pattern: t.pattern.clone(),
            period: Duration::from_secs_f64(60.0 / cfg.bpm as f64 / t.div as f64),
            next_time: now,
            token_index: 0,
            muted: t.muted,
        })
        .collect()
}

fn merge_runtime_preserving_phase(
    old_cfg: &SequencerConfig,
    new_cfg: &SequencerConfig,
    old_rt: &Vec<TrackRuntime>,
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
        let new_period = Duration::from_secs_f64(60.0 / new_cfg.bpm as f64 / t.div as f64);
        if let Some(old_rt) = name_to_rt.get(t.name.as_str()) {
            // Compute remaining time in old schedule
            let old_period = old_rt.period;
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
                period: new_period,
                next_time: new_next,
                token_index: new_token_index,
                muted: t.muted,
            });
        } else {
            // New track: schedule from next token boundary
            out.push(TrackRuntime {
                data: t.data.clone(),
                gain: t.gain,
                pattern: t.pattern.clone(),
                period: new_period,
                next_time: now + new_period,
                token_index: 0,
                muted: t.muted,
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

#[cfg(test)]
mod tests {
    use super::db_to_amplitude;

    #[test]
    fn db_to_amplitude_converts_expected_values() {
        assert!((db_to_amplitude(0.0) - 1.0).abs() < 1e-6);
        assert!(db_to_amplitude(-6.0) < 0.6);
        assert!(db_to_amplitude(6.0) > 1.9);
    }
}
