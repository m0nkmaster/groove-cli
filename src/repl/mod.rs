use std::io::Write;

use anyhow::{bail, Result};
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
                Ok(Output::Text(format!("bpm set to {}", song.bpm)))
            } else {
                bail!("usage: bpm <number>");
            }
        }
        "steps" => {
            if let Some(v) = parts.next() {
                song.steps = v.parse()?;
                Ok(Output::Text(format!("steps set to {}", song.steps)))
            } else {
                bail!("usage: steps <number>");
            }
        }
        "swing" => {
            if let Some(v) = parts.next() {
                song.swing = v.parse()?;
                Ok(Output::Text(format!("swing set to {}%", song.swing)))
            } else {
                bail!("usage: swing <percent>");
            }
        }
        "track" => {
            let name: String = parts.next().unwrap_or_default();
            if name.is_empty() { bail!("usage: track \"Name\""); }
            song.tracks.push(Track::new(name.as_str()));
            Ok(Output::Text(format!("added track {}", name)))
        }
        "pattern" => {
            let idx = parts.next().ok_or_else(|| anyhow::anyhow!("usage: pattern <track_idx> \"pattern\""))?;
            let pat = parts.next().ok_or_else(|| anyhow::anyhow!("usage: pattern <track_idx> \"pattern\""))?;
            let i: usize = idx.parse()?;
            if i == 0 || i > song.tracks.len() { bail!("no such track index"); }
            song.tracks[i - 1].pattern = Some(Pattern::visual(pat));
            Ok(Output::Text(format!("track {} pattern set", i)))
        }
        "sample" => {
            let idx = parts.next().ok_or_else(|| anyhow::anyhow!("usage: sample <track_idx> \"path\""))?;
            let p = parts.next().ok_or_else(|| anyhow::anyhow!("usage: sample <track_idx> \"path\""))?;
            let i: usize = idx.parse()?;
            if i == 0 || i > song.tracks.len() { bail!("no such track index"); }
            song.tracks[i - 1].sample = Some(p.to_string());
            Ok(Output::Text(format!("track {} sample set", i)))
        }
        "list" => Ok(Output::Text(song.list())),
        "play" => Ok(Output::Text("[play] (audio engine not yet implemented)".into())),
        "stop" => Ok(Output::Text("[stop]".into())),
        "save" => {
            let path = parts.next().ok_or_else(|| anyhow::anyhow!("usage: save \"song.toml\""))?;
            song_io::save(song, path)?;
            Ok(Output::Text("saved".into()))
        }
        "open" => {
            let path = parts.next().ok_or_else(|| anyhow::anyhow!("usage: open \"song.toml\""))?;
            let s = song_io::open(path)?;
            *song = s;
            Ok(Output::Text("opened".into()))
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
        "doc" => Ok(Output::Text("Documentation: see documentation/features/full-spec.md".into())),
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
  list                  List tracks
  play | stop           Transport (stubs for v0 scaffold)
  save "song.toml"      Save current song to TOML
  open "song.toml"      Open a song from TOML
"#;
