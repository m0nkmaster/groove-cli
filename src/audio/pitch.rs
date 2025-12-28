use std::f32::consts::PI;

use anyhow::{Context, Result};
use crate::model::track::SampleRoot;
use rodio::{Decoder, Source};
use std::fs::File;
use std::io::BufReader;

pub fn freq_to_midi_note_and_cents(freq_hz: f32) -> Option<(i32, f32)> {
    if !(freq_hz > 0.0) {
        return None;
    }
    let midi = 69.0 + 12.0 * (freq_hz / 440.0).log2();
    if !midi.is_finite() {
        return None;
    }
    let rounded = midi.round();
    let cents = (midi - rounded) * 100.0;
    Some((rounded as i32, cents))
}

pub fn midi_note_to_display(midi_note: i32) -> String {
    let pitch_class = midi_note.rem_euclid(12) as usize;
    let octave = (midi_note / 12) - 1;
    let sharps = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let flats = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
    match pitch_class {
        1 | 3 | 6 | 8 | 10 => format!(
            "{}{}/{}{octave}",
            sharps[pitch_class],
            octave,
            flats[pitch_class],
        ),
        _ => format!("{}{}", sharps[pitch_class], octave),
    }
}

pub fn analyze_sample_file(path: &str) -> Result<Option<SampleRoot>> {
    let file = File::open(path).with_context(|| format!("cannot open sample: {}", path))?;
    let reader = BufReader::new(file);
    let decoder = Decoder::new(reader).with_context(|| format!("cannot decode sample: {}", path))?;

    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels().max(1) as usize;

    // Analyze a short window from the start for speed + determinism.
    let max_seconds = 1.0_f32;
    let max_frames = (sample_rate as f32 * max_seconds).round().max(1.0) as usize;
    let max_values = max_frames.saturating_mul(channels);

    let mut mono: Vec<f32> = Vec::with_capacity(max_frames);
    let mut acc = 0.0_f32;
    let mut ch = 0usize;
    for s in decoder.convert_samples::<f32>().take(max_values) {
        acc += s;
        ch += 1;
        if ch >= channels {
            mono.push(acc / channels as f32);
            acc = 0.0;
            ch = 0;
        }
    }

    Ok(detect_pitch_autocorr(&mono, sample_rate))
}

/// Very small, deterministic pitch detector for monophonic audio.
///
/// Uses normalized autocorrelation over a bounded lag range.
pub fn detect_pitch_autocorr(mono: &[f32], sample_rate: u32) -> Option<SampleRoot> {
    if mono.len() < 256 || sample_rate == 0 {
        return None;
    }

    // Limit work; more samples doesn't meaningfully help for this simple estimator.
    let n = mono.len().min(8192);

    // Remove DC and apply a Hann window.
    let mean = mono[..n].iter().copied().sum::<f32>() / n as f32;
    let mut x = Vec::with_capacity(n);
    for (i, s) in mono[..n].iter().copied().enumerate() {
        let w = 0.5 - 0.5 * (2.0 * PI * i as f32 / (n.saturating_sub(1) as f32)).cos();
        x.push((s - mean) * w);
    }

    let energy: f32 = x.iter().map(|v| v * v).sum();
    if energy <= 1e-9 {
        return None;
    }

    // Frequency bounds.
    let min_freq = 50.0_f32;
    let max_freq = 2000.0_f32;
    let mut min_lag = (sample_rate as f32 / max_freq).floor() as usize;
    let mut max_lag = (sample_rate as f32 / min_freq).floor() as usize;
    min_lag = min_lag.max(2);
    max_lag = max_lag.min(n / 2).max(min_lag + 1);

    let mut best_lag = 0usize;
    let mut best_corr = f32::MIN;
    for lag in min_lag..=max_lag {
        let mut sum = 0.0_f32;
        // Autocorrelation for this lag.
        for i in 0..(n - lag) {
            sum += x[i] * x[i + lag];
        }
        if sum > best_corr {
            best_corr = sum;
            best_lag = lag;
        }
    }
    if best_lag == 0 {
        return None;
    }

    let confidence = (best_corr / energy).clamp(0.0, 1.0);
    if confidence < 0.2 {
        return None;
    }

    let freq_hz = sample_rate as f32 / best_lag as f32;
    let (midi_note, cents) = freq_to_midi_note_and_cents(freq_hz)?;
    Some(SampleRoot {
        freq_hz,
        midi_note,
        cents,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq_hz: f32, sample_rate: u32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * freq_hz * t).sin()
            })
            .collect()
    }

    #[test]
    fn midi_mapping_a4_is_69() {
        let (midi, cents) = freq_to_midi_note_and_cents(440.0).expect("midi");
        assert_eq!(midi, 69);
        assert!(cents.abs() < 1e-3);
    }

    #[test]
    fn enharmonic_display_shows_both_spellings() {
        // MIDI 61 is C#4 / Db4
        assert_eq!(midi_note_to_display(61), "C#4/Db4");
    }

    #[test]
    fn detects_a4_from_sine() {
        let sr = 44_100;
        let samples = sine(440.0, sr, 8192);
        let det = detect_pitch_autocorr(&samples, sr).expect("detect");
        assert_eq!(det.midi_note, 69);
        assert!((det.freq_hz - 440.0).abs() < 2.0, "freq was {}", det.freq_hz);
        assert!(det.cents.abs() < 10.0, "cents was {}", det.cents);
        assert!(det.confidence > 0.8, "conf was {}", det.confidence);
    }
}


