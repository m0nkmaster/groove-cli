use std::fs::File;
use std::io::BufReader;

use anyhow::{anyhow, Context, Result};
use rodio::{Decoder, OutputStream, Sink, Source};

use crate::model::song::Song;

/// Plays every non-muted track's sample once using the default audio output device.
///
/// This basic implementation simply mixes the decoded audio sources together
/// and blocks until playback has finished. Tracks without a configured sample
/// or with missing files are skipped.
pub fn play_song(song: &Song) -> Result<()> {
    let (_stream, stream_handle) = OutputStream::try_default().context("opening audio output")?;
    let sink = Sink::try_new(&stream_handle).context("creating audio sink")?;

    let mut appended_any = false;
    for track in &song.tracks {
        if track.mute {
            continue;
        }
        let Some(path) = &track.sample else {
            continue;
        };
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("warning: skipping track '{}': {}", track.name, e);
                continue;
            }
        };
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("decoding sample for track '{}'", track.name))?;
        // Apply gain in decibels by converting to amplitude multiplier.
        let gain = db_to_amplitude(track.gain_db);
        sink.append(source.amplify(gain));
        appended_any = true;
    }

    if !appended_any {
        return Err(anyhow!("no playable samples"));
    }

    sink.sleep_until_end();
    Ok(())
}

fn db_to_amplitude(db: f32) -> f32 {
    (10.0_f32).powf(db / 20.0)
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
