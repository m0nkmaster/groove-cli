use std::fs::File;
use std::io::{BufReader, Cursor};
use std::sync::{mpsc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use rodio::{Decoder, OutputStream, Sink, Source};

use crate::model::song::Song;

#[derive(Clone)]
pub struct SequencerConfig {
    pub bpm: u32,
    pub steps: u8,
    pub repeat: bool,
    pub tracks: Vec<LoadedTrack>,
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
        let mut step_secs = (60.0f64 / current.bpm as f64) * (4.0f64 / current.steps as f64);
        let mut step_index: usize = 0;
        let mut voices: Vec<Sink> = Vec::new();
        'outer: loop {
            // Process control messages
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ControlMsg::Stop => {
                        for s in voices.drain(..) { s.stop(); }
                        break 'outer;
                    }
                    ControlMsg::Update(new_cfg) => {
                        current = new_cfg;
                        step_secs = (60.0f64 / current.bpm as f64) * (4.0f64 / current.steps as f64);
                        // voices continue but future steps use new config
                    }
                }
            }

            // Trigger hits for this step
            for lt in &current.tracks {
                if lt.pattern.get(step_index % lt.pattern.len()).copied().unwrap_or(false) {
                    // play one-shot from in-memory data
                    let cursor = Cursor::new(lt.data.clone());
                    let reader = BufReader::new(cursor);
                    let source = match Decoder::new(reader) {
                        Ok(s) => s.amplify(lt.gain),
                        Err(e) => {
                            eprintln!("audio decode error: {}", e);
                            continue;
                        }
                    };
                    match Sink::try_new(&stream_handle) {
                        Ok(sink) => {
                            sink.append(source);
                            sink.play();
                            voices.push(sink);
                        }
                        Err(e) => eprintln!("audio error: {}", e),
                    }
                }
            }

            // Clean up finished voices
            voices.retain(|s| !s.empty());

            step_index += 1;
            if !current.repeat && step_index as u8 >= current.steps { // one bar then stop
                break 'outer;
            }
            std::thread::sleep(Duration::from_secs_f64(step_secs));
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
}

fn visual_to_steps(s: &str, steps: u8) -> Vec<bool> {
    let mut tokens: Vec<bool> = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| matches!(c, 'x' | 'X' | '1' | '*'))
        .collect();
    if tokens.is_empty() {
        return vec![false; steps as usize];
    }
    let target = steps as usize;
    if tokens.len() == target {
        return tokens;
    }
    // If shorter or longer, wrap to target length
    let mut out = Vec::with_capacity(target);
    for i in 0..target {
        out.push(tokens[i % tokens.len()]);
    }
    out
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
        if t.mute { continue; }
        let Some(path) = &t.sample else { continue; };
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: skipping track '{}': {}", t.name, e);
                continue;
            }
        };
        let pattern = match &t.pattern {
            Some(crate::model::pattern::Pattern::Visual(s)) => visual_to_steps(s, song.steps),
            None => vec![false; song.steps as usize],
        };
        tracks.push(LoadedTrack {
            name: t.name.clone(),
            data: bytes,
            gain: db_to_amplitude(t.gain_db),
            pattern,
        });
    }
    SequencerConfig { bpm: song.bpm, steps: song.steps, repeat: song.repeat_on(), tracks }
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
