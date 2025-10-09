use std::io::Write;

use anyhow::{anyhow, bail, Result};
use rustyline::{error::ReadlineError, history::DefaultHistory, Editor};

use crate::model::pattern::Pattern;
use crate::model::song::Song;
use crate::model::track::Track;
use crate::storage::song as song_io;

pub fn run_repl(song: &mut Song) -> Result<()> {
    let mut rl = Editor::<(), DefaultHistory>::new()?;
    let mut _line_no: usize = 1;
    loop {
        let prompt = format!("> ");
        match rl.readline(&prompt) {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                rl.add_history_entry(line.as_str())?;
                match handle_line(song, &line) {
                    Ok(Output::None) => {}
                    Ok(Output::Text(t)) => println!("{}", t),
                    Err(e) => eprintln!("error: {}", e),
                }
                _line_no += 1;
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("bye");
                break;
            }
            Err(err) => {
                eprintln!("repl error: {}", err);
                break;
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
enum Output {
    None,
    Text(String),
}

fn handle_line(song: &mut Song, line: &str) -> Result<Output> {
    let l = line.trim();
    if l.starts_with(':') {
        return handle_meta(song, &l[1..]);
    }

    // Simple commands for MVP scaffolding
    let mut parts = shlex::Shlex::new(l);
    let cmd: String = parts.next().unwrap_or_default();
    match cmd.as_str() {
        "bpm" => {
            if let Some(v) = parts.next() {
                song.bpm = v.parse()?;
                crate::audio::reload_song(song);
                Ok(Output::Text(format!("bpm set to {}", song.bpm)))
            } else {
                bail!("usage: bpm <number>");
            }
        }
        "steps" => {
            if let Some(v) = parts.next() {
                song.steps = v.parse()?;
                crate::audio::reload_song(song);
                Ok(Output::Text(format!("steps set to {}", song.steps)))
            } else {
                bail!("usage: steps <number>");
            }
        }
        "swing" => {
            if let Some(v) = parts.next() {
                song.swing = v.parse()?;
                crate::audio::reload_song(song);
                Ok(Output::Text(format!("swing set to {}%", song.swing)))
            } else {
                bail!("usage: swing <percent>");
            }
        }
        "track" => {
            let name: String = parts.next().unwrap_or_default();
            if name.is_empty() {
                bail!("usage: track \"Name\"");
            }
            song.tracks.push(Track::new(name.as_str()));
            crate::audio::reload_song(song);
            Ok(Output::Text(format!("added track {}", name)))
        }
        "pattern" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: pattern <track_idx> \"pattern\""))?;
            let pat = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: pattern <track_idx> \"pattern\""))?;
            let msg = {
                let (i, track) = track_mut(song, &idx)?;
                track.pattern = Some(Pattern::visual(pat));
                format!("track {} pattern set", i)
            };
            crate::audio::reload_song(song);
            Ok(Output::Text(msg))
        }
        "sample" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: sample <track_idx> \"path\""))?;
            let p = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: sample <track_idx> \"path\""))?;
            let msg = {
                let (i, track) = track_mut(song, &idx)?;
                track.sample = Some(p.to_string());
                format!("track {} sample set", i)
            };
            crate::audio::reload_song(song);
            Ok(Output::Text(msg))
        }
        "list" => Ok(Output::Text(song.list())),
        "play" => {
            crate::audio::play_song(song)?;
            Ok(Output::Text("[play]".into()))
        }
        "stop" => {
            crate::audio::stop();
            Ok(Output::Text("[stop]".into()))
        }
        "save" => {
            let path = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: save \"song.yaml\""))?;
            song_io::save(song, path)?;
            Ok(Output::Text("saved".into()))
        }
        "open" => {
            let path = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: open \"song.yaml\""))?;
            let s = song_io::open(path)?;
            *song = s;
            Ok(Output::Text("opened".into()))
        }
        "delay" => {
            let idx = parts.next().ok_or_else(|| {
                anyhow::anyhow!(
                    "usage: delay <track_idx> on|off | time \"1/4\" [fb <0..1>] [mix <0..1>]"
                )
            })?;
            let (display_idx, track) = track_mut(song, &idx)?;
            let action = parts.next().ok_or_else(|| {
                anyhow::anyhow!(
                    "usage: delay <track_idx> on|off | time \"1/4\" [fb <0..1>] [mix <0..1>]"
                )
            })?;
            match action.as_str() {
                "on" => {
                    track.delay.on = true;
                    let msg = format!("track {} delay on", display_idx);
                    let _ = track;
                    crate::audio::reload_song(song);
                    Ok(Output::Text(msg))
                }
                "off" => {
                    track.delay.on = false;
                    let msg = format!("track {} delay off", display_idx);
                    let _ = track;
                    crate::audio::reload_song(song);
                    Ok(Output::Text(msg))
                }
                "time" => {
                    let time_value = parts.next().ok_or_else(|| {
                        anyhow::anyhow!(
                            "usage: delay <track_idx> time \"1/4\" [fb <0..1>] [mix <0..1>]"
                        )
                    })?;
                    track.delay.time = time_value;
                    while let Some(param) = parts.next() {
                        match param.as_str() {
                            "fb" => {
                                let val = parts.next().ok_or_else(|| {
                                    anyhow::anyhow!("delay fb requires a value between 0.0 and 1.0")
                                })?;
                                track.delay.feedback = parse_unit_range("feedback", &val)?;
                            }
                            "mix" => {
                                let val = parts.next().ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "delay mix requires a value between 0.0 and 1.0"
                                    )
                                })?;
                                track.delay.mix = parse_unit_range("mix", &val)?;
                            }
                            other => bail!("unknown delay parameter: {}", other),
                        }
                    }
                    let msg = format!(
                        "track {} delay time {} fb{:.2} mix{:.2}",
                        display_idx, track.delay.time, track.delay.feedback, track.delay.mix
                    );
                    let _ = track;
                    crate::audio::reload_song(song);
                    Ok(Output::Text(msg))
                }
                _ => {
                    bail!("usage: delay <track_idx> on|off | time \"1/4\" [fb <0..1>] [mix <0..1>]")
                }
            }
        }
        "mute" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: mute <track_idx> [on|off]"))?;
            let (display_idx, track) = track_mut(song, &idx)?;
            match parts.next() {
                Some(state) => match state.as_str() {
                    "on" => track.mute = true,
                    "off" => track.mute = false,
                    _ => bail!("usage: mute <track_idx> [on|off]"),
                },
                None => {
                    track.mute = !track.mute;
                }
            }
            let msg = format!(
                "track {} mute {}",
                display_idx,
                if track.mute { "on" } else { "off" }
            );
            let _ = track;
            crate::audio::reload_song(song);
            Ok(Output::Text(msg))
        }
        "solo" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: solo <track_idx> [on|off]"))?;
            let (display_idx, track) = track_mut(song, &idx)?;
            match parts.next() {
                Some(state) => match state.as_str() {
                    "on" => track.solo = true,
                    "off" => track.solo = false,
                    _ => bail!("usage: solo <track_idx> [on|off]"),
                },
                None => {
                    track.solo = !track.solo;
                }
            }
            let msg = format!(
                "track {} solo {}",
                display_idx,
                if track.solo { "on" } else { "off" }
            );
            let _ = track;
            crate::audio::reload_song(song);
            Ok(Output::Text(msg))
        }
        "gain" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: gain <track_idx> <db>"))?;
            let (display_idx, track) = track_mut(song, &idx)?;
            let value = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: gain <track_idx> <db>"))?;
            track.gain_db = value.parse()?;
            let msg = format!(
                "track {} gain set to {:+.1}dB",
                display_idx, track.gain_db
            );
            let _ = track;
            crate::audio::reload_song(song);
            Ok(Output::Text(msg))
        }
        "div" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: div <track_idx> <tokens_per_beat>"))?;
            let (display_idx, track) = track_mut(song, &idx)?;
            let value = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: div <track_idx> <tokens_per_beat>"))?;
            let v: u32 = value.parse()?;
            if v == 0 || v > 64 {
                bail!("div must be in 1..64");
            }
            track.div = v;
            let msg = format!(
                "track {} div set to {} tokens/beat",
                display_idx, track.div
            );
            let _ = track;
            crate::audio::reload_song(song);
            Ok(Output::Text(msg))
        }
        "remove" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: remove <track_idx>"))?;
            let position = parse_track_index(song, &idx)?;
            let removed = song.tracks.remove(position);
            crate::audio::reload_song(song);
            Ok(Output::Text(format!(
                "removed track {} ({})",
                position + 1,
                removed.name
            )))
        }
        _ => bail!("unknown command. Try :help"),
    }
}

fn handle_meta(_song: &mut Song, meta: &str) -> Result<Output> {
    match meta.trim() {
        "help" => Ok(Output::Text(HELP.to_string())),
        "q" | "quit" | "exit" => {
            // Signal outer loop by returning EOF via error; simpler approach: print and exit process
            println!("bye");
            std::io::stdout().flush().ok();
            std::process::exit(0)
        }
        "doc" => Ok(Output::Text(
            "Docs: see documentation/user-guide/quickstart.md and documentation/development/DEVELOPMENT.md".into(),
        )),
        _ => Ok(Output::Text("unknown meta command".into())),
    }
}

const HELP: &str = r#"Commands:
  :help                 Show this help
  :q / :quit            Exit

  bpm <n>               Set tempo (e.g., 120)
  steps <n>             Set steps per bar (e.g., 16)
  swing <percent>       Set swing percent (0..100)
  track "Name"          Add a track
  sample <idx> "path"   Set sample path on track
  pattern <idx> "..."   Set visual pattern on track
  delay <idx> on|off    Toggle delay
  delay <idx> time "1/4" [fb <0..1>] [mix <0..1>]
  mute <idx> [on|off]   Toggle or set mute state
  solo <idx> [on|off]   Toggle or set solo state
  gain <idx> <db>       Set track gain in decibels
  remove <idx>          Remove a track
  list                  List tracks
  play | stop           Start/stop playback
  save "song.yaml"      Save current song to YAML
  open "song.yaml"      Open a song from YAML
"#;

fn parse_track_index(song: &Song, raw: &str) -> Result<usize> {
    let idx: usize = raw.parse()?;
    if idx == 0 || idx > song.tracks.len() {
        bail!("no such track index");
    }
    Ok(idx - 1)
}

fn track_mut<'a>(song: &'a mut Song, raw: &str) -> Result<(usize, &'a mut Track)> {
    let pos = parse_track_index(song, raw)?;
    let track = song
        .tracks
        .get_mut(pos)
        .ok_or_else(|| anyhow!("no such track index"))?;
    Ok((pos + 1, track))
}

fn parse_unit_range(label: &str, raw: &str) -> Result<f32> {
    let value: f32 = raw.parse()?;
    if !(0.0..=1.0).contains(&value) {
        bail!("{} must be between 0.0 and 1.0", label);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pattern::Pattern;

    fn song_with_track() -> Song {
        let mut song = Song::default();
        song.tracks.push(Track::new("Kick"));
        song
    }

    #[test]
    fn delay_commands_update_track() {
        let mut song = song_with_track();
        handle_line(&mut song, "delay 1 on").expect("delay on");
        assert!(song.tracks[0].delay.on);

        let output =
            handle_line(&mut song, "delay 1 time \"1/8\" fb 0.5 mix 0.25").expect("delay params");
        assert_eq!(song.tracks[0].delay.time, "1/8");
        assert_eq!(song.tracks[0].delay.feedback, 0.5);
        assert_eq!(song.tracks[0].delay.mix, 0.25);
        if let Output::Text(text) = output {
            assert!(text.contains("1/8"));
        }

        handle_line(&mut song, "delay 1 off").expect("delay off");
        assert!(!song.tracks[0].delay.on);
    }

    #[test]
    fn mute_and_solo_toggle_and_set() {
        let mut song = song_with_track();

        handle_line(&mut song, "mute 1").expect("mute toggle");
        assert!(song.tracks[0].mute);
        handle_line(&mut song, "mute 1 off").expect("mute off");
        assert!(!song.tracks[0].mute);

        handle_line(&mut song, "solo 1 on").expect("solo on");
        assert!(song.tracks[0].solo);
        handle_line(&mut song, "solo 1").expect("solo toggle");
        assert!(!song.tracks[0].solo);
    }

    #[test]
    fn gain_sets_value() {
        let mut song = song_with_track();
        handle_line(&mut song, "gain 1 -3.5").expect("gain set");
        assert_eq!(song.tracks[0].gain_db, -3.5);
    }

    #[test]
    fn remove_track_by_index() {
        let mut song = Song::default();
        song.tracks.push(Track::new("Kick"));
        song.tracks.push(Track::new("Snare"));

        let output = handle_line(&mut song, "remove 1").expect("remove track");
        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].name, "Snare");
        if let Output::Text(text) = output {
            assert!(text.contains("Kick"));
        }
    }

    #[test]
    fn list_includes_track_details() {
        let mut song = Song::default();
        let mut track = Track::new("Bass");
        track.sample = Some("samples/bass.wav".into());
        track.pattern = Some(Pattern::visual("x..."));
        track.mute = true;
        track.gain_db = -2.5;
        song.tracks.push(track);

        let list = song.list();
        assert!(list.contains("Bass"));
        assert!(list.contains("sample: samples/bass.wav"));
        assert!(list.contains("pattern: x..."));
        assert!(list.contains("mute:on"));
        assert!(list.contains("gain:-2.5dB"));
    }
}
