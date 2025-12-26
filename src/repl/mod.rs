use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as StdMutex;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use rustyline::{error::ReadlineError, history::DefaultHistory, Editor, ExternalPrinter};

use crate::ai::{AiConfig, PatternContext, suggest_patterns};
use crate::model::pattern::Pattern;
use crate::model::song::Song;
use crate::model::track::{Track, TrackPlayback};
use crate::pattern::scripted::PatternEngine;
use crate::storage::song as song_io;

mod completer;
use completer::GrooveHelper;

pub fn run_repl(song: &mut Song) -> Result<()> {
    let helper = GrooveHelper::new();
    let mut rl = Editor::<GrooveHelper, DefaultHistory>::new()?;
    rl.set_helper(Some(helper));
    
    // Install external printer so background logs don't break the input line
    if let Ok(pr) = rl.create_external_printer() {
        let lock = StdMutex::new(pr);
        set_external_printer(Some(Box::new(move |s: String| {
            if let Ok(mut g) = lock.lock() {
                let _ = g.print(s);
            }
        })));
    }
    let mut _line_no: usize = 1;
    loop {
        let prompt = "> ";
        match rl.readline(prompt) {
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
    // clear external printer on exit
    set_external_printer(None);
    Ok(())
}

#[allow(dead_code)]
enum Output {
    None,
    Text(String),
}

fn handle_line(song: &mut Song, line: &str) -> Result<Output> {
    handle_line_internal(song, line, true)
}

fn handle_line_internal(song: &mut Song, line: &str, allow_chain: bool) -> Result<Output> {
    let l = line.trim();
    if let Some(rest) = l.strip_prefix(':') {
        return handle_meta(song, rest);
    }

    if allow_chain {
        if let Some(commands) = parse_chained_commands(l) {
            let mut texts = Vec::new();
            for cmd in commands {
                match handle_line_internal(song, &cmd, false)? {
                    Output::None => {}
                    Output::Text(t) => texts.push(t),
                }
            }
            return Ok(if texts.is_empty() {
                Output::None
            } else {
                Output::Text(texts.join("\n"))
            });
        }
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
                .ok_or_else(|| anyhow::anyhow!("usage: pattern <track_idx>[.var] \"pattern\""))?;
            let pat = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: pattern <track_idx>[.var] \"pattern\""))?;
            
            // Check for variation notation: "1.a", "kick.fill"
            let (track_idx, variation) = if let Some(dot_pos) = idx.find('.') {
                let (t, v) = idx.split_at(dot_pos);
                (t.to_string(), Some(v[1..].to_string()))
            } else {
                (idx.to_string(), None)
            };
            
            let msg = {
                let (i, track) = track_mut(song, &track_idx)?;
                if let Some(var_name) = variation {
                    track.variations.insert(var_name.clone(), Pattern::visual(pat));
                    format!("track {} variation '{}' set", i, var_name)
                } else {
                    track.pattern = Some(Pattern::visual(pat));
                    format!("track {} pattern set", i)
                }
            };
            crate::audio::reload_song(song);
            Ok(Output::Text(msg))
        }
        "var" => {
            // Switch track to a different variation: var <track_idx> <variation_name>
            // Or: var <track_idx> (to show current variation and list available)
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: var <track_idx> [variation_name|main]"))?;
            
            if let Some(var_name) = parts.next() {
                let msg = {
                    let (i, track) = track_mut(song, &idx)?;
                    if var_name == "main" || var_name == "-" {
                        track.current_variation = None;
                        format!("track {} switched to main pattern", i)
                    } else if track.variations.contains_key(&var_name.to_string()) {
                        track.current_variation = Some(var_name.to_string());
                        format!("track {} switched to variation '{}'", i, var_name)
                    } else {
                        return Err(anyhow::anyhow!(
                            "variation '{}' not found. available: {}",
                            var_name,
                            track.variations.keys().cloned().collect::<Vec<_>>().join(", ")
                        ));
                    }
                };
                crate::audio::reload_song(song);
                Ok(Output::Text(msg))
            } else {
                // List variations
                let (i, track) = track_mut(song, &idx)?;
                let current = track.current_variation.as_deref().unwrap_or("main");
                let vars: Vec<String> = std::iter::once("main".to_string())
                    .chain(track.variations.keys().cloned())
                    .map(|v| if v == current { format!("[{}]", v) } else { v })
                    .collect();
                Ok(Output::Text(format!("track {} variations: {}", i, vars.join(", "))))
            }
        }
        "gen" => {
            // Generate a pattern using Rhai scripting
            // Usage: gen <track_idx> `script` or gen `script` (preview only)
            let first = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: gen <track_idx> `script` or gen `script`"))?;
            
            let engine = PatternEngine::new();
            
            // Check if first arg is a track index or script
            if first.starts_with('`') || first.starts_with('"') {
                // Preview mode: just evaluate and print
                let script = first.trim_matches(|c| c == '`' || c == '"');
                match engine.eval(script) {
                    Ok(pattern) => Ok(Output::Text(format!("generated: {}", pattern))),
                    Err(e) => Err(anyhow::anyhow!("script error: {}", e)),
                }
            } else {
                // Apply to track
                let script = parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("usage: gen <track_idx> `script`"))?;
                let script = script.trim_matches(|c| c == '`' || c == '"');
                
                match engine.eval(script) {
                    Ok(pattern) => {
                        let (i, track) = track_mut(song, &first)?;
                        track.pattern = Some(Pattern::visual(&pattern));
                        crate::audio::reload_song(song);
                        Ok(Output::Text(format!("track {} pattern: {}", i, pattern)))
                    }
                    Err(e) => Err(anyhow::anyhow!("script error: {}", e)),
                }
            }
        }
        "ai" => {
            // AI-powered pattern suggestions
            // Usage: ai [track_idx] "description"
            let first = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: ai [track_idx] \"description\""))?;
            
            // Collect remaining args as description
            let (track_idx, description) = if first.starts_with('"') || first.contains(' ') {
                // No track index, just description
                let desc: String = std::iter::once(first.to_string())
                    .chain(parts.map(|s| s.to_string()))
                    .collect::<Vec<_>>()
                    .join(" ");
                (None, desc.trim_matches('"').to_string())
            } else {
                // Track index provided
                let desc = parts
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                (Some(first.to_string()), desc.trim_matches('"').to_string())
            };
            
            let config = AiConfig::default();
            let context = PatternContext {
                bpm: song.bpm,
                steps: 16,
                genre_hint: None,
                other_patterns: song.tracks
                    .iter()
                    .filter_map(|t| t.active_pattern())
                    .filter_map(|p| match p {
                        Pattern::Visual(s) => Some(s.clone()),
                    })
                    .take(3)
                    .collect(),
            };
            
            println!("Generating patterns for '{}'...", description);
            
            match suggest_patterns(&config, &description, &context) {
                Ok(patterns) => {
                    let mut output = String::from("Suggestions:\n");
                    for (i, pat) in patterns.iter().enumerate() {
                        output.push_str(&format!("  {}) {}\n", i + 1, pat));
                    }
                    
                    if let Some(idx) = track_idx {
                        output.push_str(&format!("\nTo apply: pattern {} \"<pattern>\"\n", idx));
                    }
                    
                    Ok(Output::Text(output))
                }
                Err(e) => Err(anyhow::anyhow!("AI generation failed: {}", e)),
            }
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
        "playback" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: playback <track_idx> <mode>"))?;
            let (display_idx, track) = track_mut(song, &idx)?;
            let mode_raw = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: playback <track_idx> <mode>"))?;
            let mode = parse_playback_mode(&mode_raw)?;
            track.playback = mode;
            crate::audio::reload_song(song);
            Ok(Output::Text(format!(
                "track {} playback {}",
                display_idx,
                mode.as_str()
            )))
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
        "clear" => {
            // Clear live region state so next refresh starts from a clean slate
            if let Ok(mut h) = LAST_HEIGHT.lock() { *h = 0; }
            if let Ok(mut g) = LAST_TOKENS.lock() { *g = None; }
            // Emit ANSI clear screen + home
            print_external("\x1b[2J\x1b[H".into());
            Ok(Output::None)
        }
        _ => bail!("unknown command. Try :help"),
    }
}

fn parse_chained_commands(input: &str) -> Option<Vec<String>> {
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut pos = 0;
    let len = chars.len();
    let mut commands = Vec::new();
    let mut saw_separator = false;
    let mut saw_parens = false;

    while pos < len {
        skip_whitespace(input, &chars, &mut pos);
        if pos >= len {
            break;
        }

        let name_start = pos;
        while pos < len && is_ident_char(chars[pos].1) {
            pos += 1;
        }
        if name_start == pos {
            return None;
        }
        let name = slice_from(input, &chars, name_start, pos).to_string();

        skip_whitespace(input, &chars, &mut pos);

        let mut args = Vec::new();
        if pos < len && chars[pos].1 == '(' {
            saw_parens = true;
            pos += 1;
            loop {
                skip_whitespace(input, &chars, &mut pos);
                if pos >= len {
                    return None;
                }
                if chars[pos].1 == ')' {
                    pos += 1;
                    break;
                }

                let arg = if chars[pos].1 == '"' {
                    parse_string_arg(input, &chars, &mut pos)?
                } else {
                    parse_bare_arg(input, &chars, &mut pos)?
                };
                args.push(arg);

                skip_whitespace(input, &chars, &mut pos);
                if pos >= len {
                    return None;
                }
                match chars[pos].1 {
                    ',' => {
                        pos += 1;
                        continue;
                    }
                    ')' => {
                        pos += 1;
                        break;
                    }
                    _ => return None,
                }
            }
        }

        let mut command = name;
        if !args.is_empty() {
            command.push(' ');
            command.push_str(&args.join(" "));
        }
        commands.push(command);

        skip_whitespace(input, &chars, &mut pos);
        if pos < len {
            if chars[pos].1 == '.' {
                saw_separator = true;
                pos += 1;
            } else {
                return None;
            }
        }
    }

    if commands.is_empty() || !(saw_separator || saw_parens) {
        None
    } else {
        Some(commands)
    }
}

fn skip_whitespace(_input: &str, chars: &[(usize, char)], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].1.is_whitespace() {
        *pos += 1;
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c.is_ascii_digit()
}

fn parse_string_arg(input: &str, chars: &[(usize, char)], pos: &mut usize) -> Option<String> {
    let start_byte = chars[*pos].0;
    *pos += 1;
    while *pos < chars.len() {
        match chars[*pos].1 {
            '"' => {
                *pos += 1;
                let end_byte = if *pos < chars.len() {
                    chars[*pos].0
                } else {
                    input.len()
                };
                return Some(input[start_byte..end_byte].to_string());
            }
            '\\' => {
                *pos += 1;
                if *pos >= chars.len() {
                    return None;
                }
                *pos += 1;
            }
            _ => {
                *pos += 1;
            }
        }
    }
    None
}

fn parse_bare_arg(input: &str, chars: &[(usize, char)], pos: &mut usize) -> Option<String> {
    let start = *pos;
    while *pos < chars.len() {
        match chars[*pos].1 {
            ',' | ')' => break,
            c if c.is_whitespace() => break,
            _ => *pos += 1,
        }
    }
    if start == *pos {
        return None;
    }
    let slice = slice_from(input, chars, start, *pos).trim().to_string();
    if slice.is_empty() {
        None
    } else {
        Some(slice)
    }
}

fn slice_from<'a>(input: &'a str, chars: &[(usize, char)], start: usize, end: usize) -> &'a str {
    let start_byte = chars[start].0;
    let end_byte = if end < chars.len() { chars[end].0 } else { input.len() };
    &input[start_byte..end_byte]
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
        "live" => Ok(Output::Text(format!(
            "live view: {}",
            if live_view_enabled() { "on" } else { "off" }
        ))),
        "live on" => {
            set_live_view(true);
            ensure_live_ticker();
            // force first render
            if let Ok(mut g) = LAST_TOKENS.lock() { *g = None; }
            Ok(Output::Text("live view on".into()))
        }
        "live off" => {
            set_live_view(false);
            if let Ok(mut g) = LAST_TOKENS.lock() { *g = None; }
            // Clear previously drawn region
            if let Ok(mut h) = LAST_HEIGHT.lock() {
                let prev = *h;
                if prev > 0 {
                    let mut clear = String::new();
                    for _ in 0..prev { clear.push_str("\x1b[1F\x1b[2K\r"); }
                    *h = 0;
                    print_external(clear);
                }
            }
            if let Ok(mut p) = PREV_PLAYING.lock() { *p = None; }
            if let Ok(mut s) = LAST_SNAPSHOT.lock() { *s = None; }
            Ok(Output::Text("live view off".into()))
        }
        _ => Ok(Output::Text("unknown meta command".into())),
    }
}

const HELP: &str = r#"Commands:
  :help                 Show this help
  :q / :quit            Exit
  :live [on|off]        Toggle or show live playing view

Song:
  bpm <n>               Set tempo (e.g., 120)
  steps <n>             Set steps per bar (e.g., 16)
  swing <percent>       Set swing percent (0..100)
  play | stop           Start/stop playback
  save "file.yaml"      Save song to YAML
  open "file.yaml"      Load song from YAML

Tracks:
  track "Name"          Add a track
  remove <idx>          Remove a track
  list                  List all tracks
  sample <idx> "path"   Set sample (Tab for autocomplete)
  pattern <idx> "..."   Set pattern (x=hit, .=rest)
  mute <idx> [on|off]   Toggle mute
  solo <idx> [on|off]   Toggle solo
  gain <idx> <db>       Set gain in dB
  playback <idx> <mode> Set mode: gate|mono|one_shot
  div <idx> <n>         Set step division (4=16ths)

Variations:
  pattern <idx>.<var> "..."   Set named variation
  var <idx> [name|main]       Switch variation

Delay:
  delay <idx> on|off          Toggle delay
  delay <idx> time <t>        Set time (1/4, 1/8, 100ms)
  delay <idx> feedback <f>    Set feedback (0.0-1.0)
  delay <idx> mix <m>         Set wet/dry mix (0.0-1.0)

Generators:
  gen <idx> `script`    Generate pattern with Rhai
                        Built-ins: euclid(k,n), random(d,seed), fill(n)
  ai "description"      AI pattern suggestions (requires Ollama)

Pattern notation: x=hit X=accent .=rest _=tie
  Modifiers: +7 (pitch) v80 (velocity) ?50% (prob) {3} (ratchet)
  Chords: (x x+4 x+7)   Gates: =3/4   Repeats: (x.)*4

  clear                 Clear terminal
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

fn parse_playback_mode(raw: &str) -> Result<TrackPlayback> {
    match raw {
        "one_shot" | "oneshot" | "one-shot" => Ok(TrackPlayback::OneShot),
        "mono" | "monophonic" | "replace" => Ok(TrackPlayback::Mono),
        "gate" | "clip" | "hold" => Ok(TrackPlayback::Gate),
        _ => bail!("playback mode must be gate, mono, or one_shot"),
    }
}

// --- Live Playing View Toggle ---
static LIVE_VIEW: AtomicBool = AtomicBool::new(false);

fn set_live_view(on: bool) {
    LIVE_VIEW.store(on, Ordering::SeqCst);
}

pub(crate) fn live_view_enabled() -> bool {
    LIVE_VIEW.load(Ordering::SeqCst)
}

// ANSI helpers for simple highlighting
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

#[cfg(test)]
fn render_live_grid(song: &Song, snap: &crate::audio::LiveSnapshot) -> String {
    // Map track name -> (pattern bits, token index)
    // Render in the order of song.tracks
    let mut out = String::new();
    if snap.tracks.is_empty() { return out; }
    out.push_str("Tracks:\n");
    for (i, t) in song.tracks.iter().enumerate() {
        if let Some(st) = snap.tracks.iter().find(|lt| lt.name == t.name) {
            // Build a visual string from pattern bits and highlight current index
            let mut parts = Vec::with_capacity(st.pattern.len());
            for (idx, &hit) in st.pattern.iter().enumerate() {
                let ch = if hit { 'x' } else { '.' };
                if idx == st.token_index {
                    parts.push(format!("{}{}{}", GREEN, ch, RESET));
                } else {
                    parts.push(ch.to_string());
                }
            }
            let line = format!("{} {:<6} | {}\n", i + 1, t.name, parts.join(" "));
            out.push_str(&line);
        }
    }
    out
}

fn render_live_grid_from_snapshot(snap: &crate::audio::LiveSnapshot) -> String {
    let mut out = String::new();
    if snap.tracks.is_empty() { return out; }
    out.push_str("Tracks:\n");
    for (i, st) in snap.tracks.iter().enumerate() {
        let mut parts = Vec::with_capacity(st.pattern.len());
        for (idx, &hit) in st.pattern.iter().enumerate() {
            let ch = if hit { 'x' } else { '.' };
            if idx == st.token_index {
                parts.push(format!("{}{}{}", GREEN, ch, RESET));
            } else {
                parts.push(ch.to_string());
            }
        }
        let line = format!("{} {:<6} | {}\n", i + 1, st.name, parts.join(" "));
        out.push_str(&line);
    }
    out
}

// ---------------- Live ticker -----------------
static TICKER_STARTED: AtomicBool = AtomicBool::new(false);
type TokensState = Option<Vec<(String, usize)>>;
static LAST_TOKENS: once_cell::sync::Lazy<StdMutex<TokensState>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(None));
static LAST_HEIGHT: once_cell::sync::Lazy<StdMutex<usize>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(0));
static PREV_PLAYING: once_cell::sync::Lazy<StdMutex<Option<bool>>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(None));
static LAST_SNAPSHOT: once_cell::sync::Lazy<StdMutex<Option<crate::audio::LiveSnapshot>>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(None));

type PrinterFn = Box<dyn Fn(String) + Send + Sync + 'static>;
static EXTERNAL_PRINTER: once_cell::sync::Lazy<StdMutex<Option<PrinterFn>>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(None));

fn set_external_printer(p: Option<PrinterFn>) {
    let mut guard = EXTERNAL_PRINTER.lock().unwrap();
    *guard = p;
}

fn print_external(s: String) {
    if let Some(ref f) = *EXTERNAL_PRINTER.lock().unwrap() {
        f(s);
    } else {
        println!("{}", s);
    }
}

fn ensure_live_ticker() {
    if TICKER_STARTED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        thread::spawn(|| {
            loop {
                if !live_view_enabled() { break; }
                let playing = crate::audio::is_playing();
                let status_changed;
                {
                    let mut prev = PREV_PLAYING.lock().unwrap();
                    status_changed = (*prev).map(|p| p != playing).unwrap_or(true);
                    *prev = Some(playing);
                }

                let mut snap_opt = crate::audio::snapshot_live_state();
                if let Some(ref s) = snap_opt { if let Ok(mut g) = LAST_SNAPSHOT.lock() { *g = Some(s.clone()); } }
                if snap_opt.is_none() {
                    if let Ok(g) = LAST_SNAPSHOT.lock() { snap_opt = g.clone(); }
                }

                if let Some(snap) = snap_opt {
                    let tokens: Vec<(String, usize)> = snap
                        .tracks
                        .iter()
                        .map(|t| (t.name.clone(), t.token_index))
                        .collect();
                    let mut guard = LAST_TOKENS.lock().unwrap();
                    let tokens_changed = match &*guard {
                        Some(prev) => prev != &tokens,
                        None => true,
                    };
                    if tokens_changed || status_changed {
                        *guard = Some(tokens);
                        let header = format!(
                            "[live] status:{}",
                            if playing { "playing" } else { "stopped" }
                        );
                        let mut lines = vec![header];
                        let grid = render_live_grid_from_snapshot(&snap);
                        if !grid.is_empty() { lines.extend(grid.lines().map(|s| s.to_string())); }
                        print_live_region(lines);
                    }
                } else if status_changed {
                    // No snapshot but status changed: print header only
                    let header = format!(
                        "[live] status:{}",
                        if playing { "playing" } else { "stopped" }
                    );
                    print_live_region(vec![header]);
                }
                thread::sleep(Duration::from_millis(250));
            }
            TICKER_STARTED.store(false, Ordering::SeqCst);
        });
    }
}

fn print_live_region(lines: Vec<String>) {
    let mut msg = String::new();
    // Determine previous height and build clear+overwrite commands
    let mut last_h = LAST_HEIGHT.lock().unwrap();
    let prev = *last_h;
    if prev > 0 {
        // Move cursor up prev lines and clear each line
        for _ in 0..prev {
            msg.push_str("\x1b[1F\x1b[2K\r");
        }
    } else {
        // Ensure a clean separation before first render
        msg.push('\n');
    }
    // Write new lines
    for (i, l) in lines.iter().enumerate() {
        if i > 0 { msg.push('\n'); }
        msg.push_str(l);
    }
    // Track new height for next refresh
    *last_h = lines.len();
    drop(last_h);
    print_external(msg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pattern::Pattern;
    use crate::model::track::TrackPlayback;
    use crate::audio::{LiveSnapshot, LiveTrackSnapshot};

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
    fn playback_command_sets_mode() {
        let mut song = song_with_track();
        handle_line(&mut song, "playback 1 mono").expect("playback set");
        assert_eq!(song.tracks[0].playback, TrackPlayback::Mono);
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
        assert!(list.contains("playback:gate"));
    }

    #[test]
    fn live_toggle_meta_commands() {
        let mut song = Song::default();
        // default off
        assert!(!live_view_enabled());

        // query
        if let Output::Text(t) = handle_line(&mut song, ":live").expect("meta live") {
            assert!(t.contains("off"));
        } else { panic!("expected text"); }

        // turn on
        if let Output::Text(t) = handle_line(&mut song, ":live on").expect(":live on") {
            assert!(t.contains("on"));
        } else { panic!("expected text"); }
        assert!(live_view_enabled());

        // turn off
        if let Output::Text(t) = handle_line(&mut song, ":live off").expect(":live off") {
            assert!(t.contains("off"));
        } else { panic!("expected text"); }
        assert!(!live_view_enabled());
    }

    #[test]
    fn render_live_grid_highlights_playhead() {
        let mut song = Song::default();
        let mut t1 = Track::new("Kick");
        t1.pattern = Some(Pattern::visual("x . x ."));
        let mut t2 = Track::new("Snare");
        t2.pattern = Some(Pattern::visual(". . x ."));
        song.tracks.push(t1);
        song.tracks.push(t2);

        let snap = LiveSnapshot {
            tracks: vec![
                LiveTrackSnapshot { name: "Kick".into(), token_index: 2, pattern: vec![true,false,true,false] },
                LiveTrackSnapshot { name: "Snare".into(), token_index: 2, pattern: vec![false,false,true,false] },
            ]
        };

        let grid = render_live_grid(&song, &snap);
        assert!(grid.contains("Tracks:"));
        // Expect a green-highlighted 'x' for the playhead positions
        assert!(grid.contains("\x1b[32mx\x1b[0m"));
        // Ensure both tracks are present in order
        assert!(grid.contains("1 Kick"));
        assert!(grid.contains("2 Snare"));
    }

    #[test]
    fn render_live_grid_from_snapshot_order_and_colors() {
        let snap = LiveSnapshot {
            tracks: vec![
                LiveTrackSnapshot { name: "A".into(), token_index: 0, pattern: vec![true,false] },
                LiveTrackSnapshot { name: "B".into(), token_index: 1, pattern: vec![true,true] },
            ]
        };
        let out = render_live_grid_from_snapshot(&snap);
        assert!(out.contains("1 A"));
        assert!(out.contains("2 B"));
        // Has green highlight sequences
        assert!(out.contains("\x1b[32m"));
    }

    #[test]
    fn clear_command_resets_live_region_height() {
        // Pretend something was rendered previously
        if let Ok(mut h) = LAST_HEIGHT.lock() { *h = 5; }
        let mut song = Song::default();
        // Should not error
        handle_line(&mut song, "clear").expect("clear");
        // Height should be reset so next render starts fresh
        if let Ok(h) = LAST_HEIGHT.lock() { assert_eq!(*h, 0); }
    }

    #[test]
    fn paren_syntax_executes_single_command() {
        let mut song = Song::default();
        let output = handle_line(&mut song, "track(\"Kick\")").expect("track");

        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].name, "Kick");
        if let Output::Text(t) = output {
            assert!(t.contains("Kick"));
        } else { panic!("expected text output"); }
    }

    #[test]
    fn chaining_executes_commands_in_order() {
        let mut song = Song::default();
        let output = handle_line(
            &mut song,
            "track(\"Kick\").sample(1, \"samples/909/kick.wav\").pattern(1, \"x...\")",
        )
        .expect("chain executes");

        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].name, "Kick");
        assert_eq!(song.tracks[0].sample.as_deref(), Some("samples/909/kick.wav"));
        match &song.tracks[0].pattern {
            Some(Pattern::Visual(pat)) => assert_eq!(pat, "x..."),
            other => panic!("unexpected pattern state: {:?}", other),
        }

        if let Output::Text(t) = output {
            assert!(t.contains("pattern set"));
            assert!(t.contains("sample set"));
        } else { panic!("expected text output"); }
    }
}
