use std::io::Write;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as StdMutex;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use rustyline::{error::ReadlineError, history::DefaultHistory, Editor, ExternalPrinter};

use crate::ai::{AiConfig, DelayInfo, PatternContext, TrackInfo, suggest_patterns};
use crate::model::pattern::Pattern;
use crate::model::song::Song;
use crate::model::track::{Track, TrackPlayback};
use crate::pattern::scripted::PatternEngine;
use crate::storage::song as song_io;

pub mod completer;
mod browser;
pub mod style;

use completer::GrooveHelper;
use browser::{browse_samples, BrowserResult};
use style::*;

pub fn run_repl(song: &mut Song) -> Result<()> {
    let helper = GrooveHelper::new();
    let mut rl = Editor::<GrooveHelper, DefaultHistory>::new()?;
    rl.set_helper(Some(helper));
    
    // Initialize track names for autocomplete
    update_track_names(song);
    
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
        let prompt = format_prompt(song.bpm, crate::audio::is_playing());
        match rl.readline(&prompt) {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                rl.add_history_entry(line.as_str())?;
                match handle_line(song, &line) {
                    Ok(Output::None) => {}
                    Ok(Output::Text(t)) => println!("{}", t),
                    Err(e) => eprintln!("{}", e),
                }
                _line_no += 1;
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("{}", goodbye());
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

enum Output {
    None,
    Text(String),
}

/// Handle a command line from the TUI (public API).
/// Returns Ok(Some(text)) for output, Ok(None) for no output, Err for errors.
pub fn handle_line_for_tui(song: &mut Song, line: &str) -> Result<Option<String>> {
    match handle_line(song, line) {
        Ok(Output::None) => Ok(None),
        Ok(Output::Text(t)) => Ok(Some(t)),
        Err(e) => Err(e),
    }
}

fn handle_line(song: &mut Song, line: &str) -> Result<Output> {
    handle_line_internal(song, line, true)
}

fn handle_line_internal(song: &mut Song, line: &str, allow_chain: bool) -> Result<Output> {
    let l = line.trim();

    // Semicolon-separated commands (run left-to-right; stop on first error).
    // This is handled before meta commands so `:live on; go` works as expected.
    if allow_chain {
        if let Some(commands) = parse_semicolon_commands(l) {
            let mut texts = Vec::new();
            for cmd in commands {
                match handle_line_internal(song, &cmd, true)? {
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
    
    // Meta commands
    if let Some(rest) = l.strip_prefix(':') {
        return handle_meta(song, rest);
    }
    
    // Help shortcut
    if l == "?" {
        return Ok(Output::Text(help_box()));
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

    // Check for new-style commands first
    
    // Just a number = set BPM
    if let Ok(bpm) = l.parse::<u32>() {
        if bpm > 0 && bpm <= 999 {
            song.bpm = bpm;
            crate::audio::reload_song(song);
            return Ok(Output::Text(tempo(bpm)));
        }
    }
    
    // + name [name...] = add track(s)
    if let Some(rest) = l.strip_prefix('+').map(|s| s.trim()) {
        if rest.is_empty() {
            bail!("usage: + name [name...]");
        }

        let names: Vec<&str> = rest.split_whitespace().collect();
        if names.is_empty() {
            bail!("usage: + name [name...]");
        }

        // Validate atomically before mutating the song.
        let mut seen: HashSet<String> = HashSet::new();
        for name in &names {
            let name_lower = name.to_lowercase();
            if !seen.insert(name_lower.clone()) {
                bail!("  {} track \"{}\" already exists", EMOJI_THINK, name);
            }
            if song
                .tracks
                .iter()
                .any(|t| t.name.to_lowercase() == name_lower)
            {
                bail!("  {} track \"{}\" already exists", EMOJI_THINK, name);
            }
        }

        for name in &names {
            song.tracks.push(Track::new(*name));
        }
        crate::audio::reload_song(song);
        update_track_names(song);
        return Ok(Output::Text(success(&format!("added {}", names.join(" ")))));
    }
    
    // - trackname = remove track
    if let Some(name) = l.strip_prefix('-').map(|s| s.trim()) {
        if !name.is_empty() {
            let position = parse_track_index(song, name)?;
            let removed = song.tracks.remove(position);
            crate::audio::reload_song(song);
            update_track_names(song);
            return Ok(Output::Text(success(&format!("removed {}", removed.name))));
        }
    }
    
    // Try track-first syntax: trackname ...
    if let Some(result) = try_track_first_command(song, l)? {
        return Ok(result);
    }

    // Simple commands for MVP scaffolding
    let mut parts = shlex::Shlex::new(l);
    let cmd: String = parts.next().unwrap_or_default();
    match cmd.as_str() {
        // New aliases
        "go" => {
            crate::audio::play_song(song)?;
            return Ok(Output::Text(playing()));
        }
        "." => {
            crate::audio::stop();
            return Ok(Output::Text(stopped()));
        }
        "ls" => {
            return Ok(Output::Text(format_track_list(song)));
        }
        "bpm" => {
            if let Some(v) = parts.next() {
                song.bpm = v.parse()?;
                crate::audio::reload_song(song);
                Ok(Output::Text(tempo(song.bpm)))
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
                bail!("usage: + trackname");
            }
            if name.contains(' ') {
                bail!("  {} track names must be single words (no spaces)", EMOJI_THINK);
            }
            // Check for duplicate
            let name_lower = name.to_lowercase();
            if song.tracks.iter().any(|t| t.name.to_lowercase() == name_lower) {
                bail!("  {} track \"{}\" already exists", EMOJI_THINK, name);
            }
            song.tracks.push(Track::new(name.as_str()));
            crate::audio::reload_song(song);
            update_track_names(song);
            Ok(Output::Text(success(&format!("added {}", name))))
        }
        "pattern" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname x...x..."))?;
            let pat = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname x...x..."))?;
            
            // Check for variation notation: "1.a", "kick.fill"
            let (track_idx, variation) = if let Some(dot_pos) = idx.find('.') {
                let (t, v) = idx.split_at(dot_pos);
                (t.to_string(), Some(v[1..].to_string()))
            } else {
                (idx.to_string(), None)
            };
            
            let (name, pat_display) = {
                let (_, track) = track_mut(song, &track_idx)?;
                let track_name = track.name.clone();
                if let Some(var_name) = variation {
                    track.variations.insert(var_name.clone(), Pattern::visual(&pat));
                    (format!("{}.{}", track_name, var_name), pat)
                } else {
                    track.pattern = Some(Pattern::visual(&pat));
                    (track_name, pat)
                }
            };
            crate::audio::reload_song(song);
            Ok(Output::Text(track_pattern(&name, &pat_display)))
        }
        "var" => {
            // Switch track to a different variation: var <track_idx> <variation_name>
            // Or: var <track_idx> (to show current variation and list available)
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname > variation"))?;
            
            if let Some(var_name) = parts.next() {
                let msg = {
                    let (_, track) = track_mut(song, &idx)?;
                    let name = track.name.clone();
                    if var_name == "main" || var_name == "-" {
                        track.current_variation = None;
                        track_variation(&name, "main")
                    } else if track.variations.contains_key(&var_name.to_string()) {
                        track.current_variation = Some(var_name.to_string());
                        track_variation(&name, &var_name)
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
                let (_, track) = track_mut(song, &idx)?;
                let name = track.name.clone();
                let current = track.current_variation.as_deref().unwrap_or("main");
                let vars: Vec<String> = std::iter::once("main".to_string())
                    .chain(track.variations.keys().cloned())
                    .map(|v| if v == current { format!("[{}]", v) } else { v })
                    .collect();
                Ok(Output::Text(format!("  {}  variations: {}", name, vars.join(", "))))
            }
        }
        "gen" => {
            // Generate a pattern using Rhai scripting
            // Usage: gen <track_idx> `script` or gen `script` (preview only)
            let first = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname gen euclid(5,16)"))?;
            
            let engine = PatternEngine::new();
            
            // Check if first arg is a track index or script
            if first.starts_with('`') || first.starts_with('"') {
                // Preview mode: just evaluate and print
                let script = first.trim_matches(|c| c == '`' || c == '"');
                match engine.eval(script) {
                    Ok(pattern) => Ok(Output::Text(track_generated("", &pattern))),
                    Err(e) => Err(anyhow::anyhow!("script error: {}", e)),
                }
            } else {
                // Apply to track
                let script = parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("usage: trackname gen euclid(5,16)"))?;
                let script = script.trim_matches(|c| c == '`' || c == '"');
                
                match engine.eval(script) {
                    Ok(pattern) => {
                        let (_, track) = track_mut(song, &first)?;
                        let name = track.name.clone();
                        track.pattern = Some(Pattern::visual(&pattern));
                        crate::audio::reload_song(song);
                        Ok(Output::Text(track_generated(&name, &pattern)))
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
                target_track: track_idx.clone().unwrap_or_default(),
                tracks: song.tracks.iter().map(|t| TrackInfo {
                    name: t.name.clone(),
                    sample: t.sample.clone(),
                    pattern: t.active_pattern().map(|p| match p {
                        Pattern::Visual(s) => s.clone(),
                    }),
                    variations: t.variations.keys().cloned().collect(),
                    current_variation: t.current_variation.clone(),
                    muted: t.mute,
                    solo: t.solo,
                    gain_db: t.gain_db,
                    delay: if t.delay.on {
                        Some(DelayInfo {
                            on: t.delay.on,
                            time: t.delay.time.clone(),
                            feedback: t.delay.feedback,
                            mix: t.delay.mix,
                        })
                    } else {
                        None
                    },
                }).collect(),
            };
            
            // Note: No println here - it corrupts TUI display
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
                .ok_or_else(|| anyhow::anyhow!("usage: trackname ~ samplepath"))?;
            let p = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname ~ samplepath"))?;
            
            // Resolve sample path (supports shortcuts like "909/kick")
            let resolved = resolve_sample_path(&p);
            
            // Validate sample exists
            let path = std::path::Path::new(&resolved);
            if !path.is_file() {
                if path.exists() {
                    return Err(anyhow::anyhow!("sample must be a file: {}", resolved));
                }
                // Try to find similar samples
                let suggestions = find_similar_samples(&p);
                return Err(anyhow::anyhow!("{}", not_found_sample(&p, &suggestions)));
            }
            
            let name = {
                let (_, track) = track_mut(song, &idx)?;
                let track_name = track.name.clone();
                track.sample = Some(resolved.clone());
                track_name
            };
            crate::audio::reload_song(song);
            Ok(Output::Text(track_sample(&name, &resolved)))
        }
        "samples" => {
            // List available samples with optional filter
            let filter = parts.next();
            let samples = list_available_samples(filter.as_deref());
            if samples.is_empty() {
                Ok(Output::Text("no samples found (add .wav/.mp3 files to samples/)".into()))
            } else {
                Ok(Output::Text(samples))
            }
        }
        "preview" => {
            // Preview a sample without setting it
            let p = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: preview \"path\""))?;
            let resolved = resolve_sample_path(&p);
            let path = std::path::Path::new(&resolved);
            if !path.is_file() {
                if path.exists() {
                    return Err(anyhow::anyhow!("sample must be a file: {}", resolved));
                }
                return Err(anyhow::anyhow!("sample not found: {}", resolved));
            }
            crate::audio::preview_sample(&resolved)?;
            Ok(Output::Text(format!("▶ {}", resolved)))
        }
        "browse" => {
            // Interactive sample browser
            let start_dir = parts.next().unwrap_or("samples".to_string());
            match browse_samples(&start_dir) {
                Ok(BrowserResult::Selected(path)) => {
                    // If a track index follows, set the sample
                    if let Some(idx) = parts.next() {
                        if !std::path::Path::new(&path).is_file() {
                            return Err(anyhow::anyhow!("sample must be a file: {}", path));
                        }
                        let (i, track) = track_mut(song, &idx)?;
                        track.sample = Some(path.clone());
                        crate::audio::reload_song(song);
                        Ok(Output::Text(format!("track {} sample: {}", i, path)))
                    } else {
                        // Just return the selected path
                        Ok(Output::Text(format!("selected: {}", path)))
                    }
                }
                Ok(BrowserResult::Cancelled) => Ok(Output::Text("cancelled".into())),
                Err(e) => Err(anyhow::anyhow!("browser error: {}", e)),
            }
        }
        "list" => Ok(Output::Text(format_track_list(song))),
        "play" => {
            crate::audio::play_song(song)?;
            Ok(Output::Text(playing()))
        }
        "stop" => {
            crate::audio::stop();
            Ok(Output::Text(stopped()))
        }
        "save" => {
            let path = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: save song.yaml"))?;
            song_io::save(song, &path)?;
            Ok(Output::Text(saved(&path)))
        }
        "open" => {
            let path = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: open song.yaml"))?;
            let s = song_io::open(&path)?;
            *song = s;
            update_track_names(song);
            Ok(Output::Text(opened(&path)))
        }
        "delay" => {
            let idx = parts.next().ok_or_else(|| {
                anyhow::anyhow!("usage: trackname delay on|off or trackname delay 1/8 0.4 0.3")
            })?;
            let (_, track) = track_mut(song, &idx)?;
            let name = track.name.clone();
            let action = parts.next().ok_or_else(|| {
                anyhow::anyhow!("usage: trackname delay on|off or trackname delay 1/8 0.4 0.3")
            })?;
            match action.as_str() {
                "on" => {
                    track.delay.on = true;
                    crate::audio::reload_song(song);
                    Ok(Output::Text(track_delay(&name, true, None, None, None)))
                }
                "off" => {
                    track.delay.on = false;
                    crate::audio::reload_song(song);
                    Ok(Output::Text(track_delay(&name, false, None, None, None)))
                }
                "time" => {
                    let time_value = parts.next().ok_or_else(|| {
                        anyhow::anyhow!("usage: trackname delay 1/8 0.4 0.3")
                    })?;
                    track.delay.on = true;
                    track.delay.time = time_value.clone();
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
                                    anyhow::anyhow!("delay mix requires a value between 0.0 and 1.0")
                                })?;
                                track.delay.mix = parse_unit_range("mix", &val)?;
                            }
                            other => bail!("unknown delay parameter: {}", other),
                        }
                    }
                    let t = track.delay.time.clone();
                    let fb = track.delay.feedback;
                    let mix = track.delay.mix;
                    crate::audio::reload_song(song);
                    Ok(Output::Text(track_delay(&name, true, Some(&t), Some(fb), Some(mix))))
                }
                // New syntax: trackname delay 1/8 0.4 0.3
                time_val => {
                    track.delay.on = true;
                    track.delay.time = time_val.to_string();
                    if let Some(fb) = parts.next() {
                        track.delay.feedback = parse_unit_range("feedback", &fb)?;
                    }
                    if let Some(mix) = parts.next() {
                        track.delay.mix = parse_unit_range("mix", &mix)?;
                    }
                    let t = track.delay.time.clone();
                    let fb = track.delay.feedback;
                    let mix = track.delay.mix;
                    crate::audio::reload_song(song);
                    Ok(Output::Text(track_delay(&name, true, Some(&t), Some(fb), Some(mix))))
                }
            }
        }
        "mute" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname mute"))?;
            let (_, track) = track_mut(song, &idx)?;
            let name = track.name.clone();
            match parts.next() {
                Some(state) => match state.as_str() {
                    "on" => track.mute = true,
                    "off" => track.mute = false,
                    _ => bail!("usage: trackname mute"),
                },
                None => {
                    track.mute = !track.mute;
                }
            }
            let is_muted = track.mute;
            crate::audio::reload_song(song);
            Ok(Output::Text(if is_muted { track_muted(&name) } else { track_unmuted(&name) }))
        }
        "unmute" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname unmute"))?;
            let (_, track) = track_mut(song, &idx)?;
            let name = track.name.clone();
            track.mute = false;
            crate::audio::reload_song(song);
            Ok(Output::Text(track_unmuted(&name)))
        }
        "solo" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname solo"))?;
            let (_, track) = track_mut(song, &idx)?;
            let name = track.name.clone();
            match parts.next() {
                Some(state) => match state.as_str() {
                    "on" => track.solo = true,
                    "off" => track.solo = false,
                    _ => bail!("usage: trackname solo"),
                },
                None => {
                    track.solo = !track.solo;
                }
            }
            let is_solo = track.solo;
            crate::audio::reload_song(song);
            Ok(Output::Text(track_solo(&name, is_solo)))
        }
        "gain" => {
            let idx = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname -3db"))?;
            let (_, track) = track_mut(song, &idx)?;
            let name = track.name.clone();
            let value = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: trackname -3db"))?;
            track.gain_db = value.parse()?;
            let db = track.gain_db;
            crate::audio::reload_song(song);
            Ok(Output::Text(track_gain(&name, db)))
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
                .ok_or_else(|| anyhow::anyhow!("usage: - trackname"))?;
            let position = parse_track_index(song, &idx)?;
            let removed = song.tracks.remove(position);
            crate::audio::reload_song(song);
            update_track_names(song);
            Ok(Output::Text(success(&format!("removed {}", removed.name))))
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

fn parse_semicolon_commands(input: &str) -> Option<Vec<String>> {
    let mut commands: Vec<String> = Vec::new();
    let mut start = 0usize;
    let mut in_quotes = false;
    let mut escape = false;
    let mut bracket_depth: u32 = 0;
    let mut paren_depth: u32 = 0;

    for (idx, ch) in input.char_indices() {
        if in_quotes {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_quotes = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_quotes = true,
            '[' => bracket_depth = bracket_depth.saturating_add(1),
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth = paren_depth.saturating_add(1),
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ';' if bracket_depth == 0 && paren_depth == 0 => {
                let part = input[start..idx].trim();
                if !part.is_empty() {
                    commands.push(part.to_string());
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        commands.push(tail.to_string());
    }

    if commands.len() >= 2 {
        Some(commands)
    } else {
        None
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
        "help" => Ok(Output::Text(help_box())),
        "q" | "quit" | "exit" => {
            // Signal outer loop by returning EOF via error; simpler approach: print and exit process
            println!("{}", goodbye());
            std::io::stdout().flush().ok();
            std::process::exit(0)
        }
        "doc" => Ok(Output::Text(
            "  Docs: see documentation/user-guide/quickstart.md".into(),
        )),
        "live" => Ok(Output::Text(live_view(live_view_enabled()))),
        "live on" => {
            set_live_view(true);
            ensure_live_ticker();
            // force first render
            if let Ok(mut g) = LAST_TOKENS.lock() { *g = None; }
            Ok(Output::Text(live_view(true)))
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
            Ok(Output::Text(live_view(false)))
        }
        _ => Ok(Output::Text("  unknown meta command. try ?".into())),
    }
}

// HELP constant removed - now using help_box() from style module

/// Get track names as a vector (for error messages)
#[allow(dead_code)]
fn get_track_names_vec(song: &Song) -> Vec<String> {
    song.tracks.iter().map(|t| t.name.clone()).collect()
}

/// Format the track list with pretty output
fn format_track_list(song: &Song) -> String {
    if song.tracks.is_empty() {
        return "  (no tracks)".to_string();
    }
    
    let mut out = String::new();
    out.push('\n');
    
    for t in &song.tracks {
        // Mute/solo indicator
        let status = if t.mute {
            EMOJI_MUTE
        } else if t.solo {
            EMOJI_SOLO
        } else {
            EMOJI_UNMUTE
        };
        
        // Pattern display
        let pattern_str = match &t.pattern {
            Some(Pattern::Visual(p)) => prettify_pattern(p),
            None => "·".repeat(16),
        };
        
        // Gain (only show if not 0)
        let gain_str = if t.gain_db != 0.0 {
            format!("  {:+.0}db", t.gain_db)
        } else {
            String::new()
        };
        
        out.push_str(&format!(
            "  {:<8} {}  {}{}\n",
            t.name, status, pattern_str, gain_str
        ));
    }
    
    out
}

/// Try to parse track-first command syntax (e.g., "kick x...x...", "kick ~ 909/kick")
fn try_track_first_command(song: &mut Song, line: &str) -> Result<Option<Output>> {
    let mut parts = shlex::Shlex::new(line);
    let first = match parts.next() {
        Some(f) => f,
        None => return Ok(None),
    };
    
    // Check for track.variation syntax
    let (track_name, variation) = if let Some(dot_pos) = first.find('.') {
        let (t, v) = first.split_at(dot_pos);
        (t.to_string(), Some(v[1..].to_string()))
    } else {
        (first.clone(), None)
    };
    
    // Check if first word is a known track name
    let track_names: Vec<String> = song.tracks.iter().map(|t| t.name.to_lowercase()).collect();
    if !track_names.contains(&track_name.to_lowercase()) {
        return Ok(None); // Not a track-first command
    }

    let tokens: Vec<String> = parts.collect();
    if tokens.is_empty() {
        return Ok(None);
    }

    // Atomic per line: apply to a clone, then commit + reload once.
    let mut draft = song.clone();
    let mut out_lines: Vec<String> = Vec::new();

    let is_boundary = |t: &str| {
        matches!(t, "mute" | "unmute" | "solo" | "delay" | "gen" | "ai" | ">")
            || looks_like_gain(t)
            || looks_like_pattern(t)
    };

    let mut i = 0usize;
    while i < tokens.len() {
        let tok = tokens[i].as_str();

        // Pattern: kick x...x...
        if looks_like_pattern(tok) {
            let pat = tok;
            let msg = {
                let (_, track) = track_mut(&mut draft, &track_name)?;
                let name = track.name.clone();
                if let Some(ref var_name) = variation {
                    track.variations.insert(var_name.clone(), Pattern::visual(pat));
                    track_pattern(&format!("{}.{}", name, var_name), pat)
                } else {
                    track.pattern = Some(Pattern::visual(pat));
                    track_pattern(&name, pat)
                }
            };
            out_lines.push(msg);
            i += 1;
            continue;
        }

        // Gain: kick -3db or kick +2db
        if looks_like_gain(tok) {
            let db = parse_gain_value(tok)?;
            let msg = {
                let (_, track) = track_mut(&mut draft, &track_name)?;
                let name = track.name.clone();
                track.gain_db = db;
                track_gain(&name, db)
            };
            out_lines.push(msg);
            i += 1;
            continue;
        }

        match tok {
            // Sample: kick ~ query... (supports spaces) OR kick ~[query with spaces]
            "~" => {
                let mut j = i + 1;
                if j >= tokens.len() {
                    return Err(anyhow!("usage: trackname ~ sample_query"));
                }

                // Collect query tokens until we hit another segment boundary.
                let mut query_parts: Vec<String> = Vec::new();
                while j < tokens.len() && !is_boundary(tokens[j].as_str()) {
                    query_parts.push(tokens[j].clone());
                    j += 1;
                }
                if query_parts.is_empty() {
                    return Err(anyhow!("usage: trackname ~ sample_query"));
                }
                let sample_query = query_parts.join(" ");
                let resolved = resolve_sample_path(&sample_query);
                let path = std::path::Path::new(&resolved);
                if !path.is_file() {
                    if path.exists() {
                        return Err(anyhow!("sample must be a file: {}", resolved));
                    }
                    let suggestions = find_similar_samples(&sample_query);
                    return Err(anyhow!("{}", not_found_sample(&sample_query, &suggestions)));
                }
                let msg = {
                    let (_, track) = track_mut(&mut draft, &track_name)?;
                    let name = track.name.clone();
                    track.sample = Some(resolved.clone());
                    track_sample(&name, &resolved)
                };
                out_lines.push(msg);
                i = j;
            }
            // Sample with ~ attached: kick ~909/kick OR kick ~[linn snare class]
            s if s.starts_with("~[") => {
                // Parse a bracketed query which may span multiple tokens: ~[a b c]
                let mut j = i;
                let mut raw_parts: Vec<String> = Vec::new();
                while j < tokens.len() {
                    raw_parts.push(tokens[j].clone());
                    if tokens[j].ends_with(']') {
                        break;
                    }
                    j += 1;
                }
                if raw_parts.is_empty() {
                    return Err(anyhow!("usage: trackname ~[sample query]"));
                }
                let raw = raw_parts.join(" ");
                let sample_query = raw
                    .trim_start_matches("~[")
                    .trim_end_matches(']')
                    .to_string();
                if sample_query.trim().is_empty() {
                    return Err(anyhow!("usage: trackname ~[sample query]"));
                }

                let resolved = resolve_sample_path(&sample_query);
                let path = std::path::Path::new(&resolved);
                if !path.is_file() {
                    if path.exists() {
                        return Err(anyhow!("sample must be a file: {}", resolved));
                    }
                    let suggestions = find_similar_samples(&sample_query);
                    return Err(anyhow!("{}", not_found_sample(&sample_query, &suggestions)));
                }
                let msg = {
                    let (_, track) = track_mut(&mut draft, &track_name)?;
                    let name = track.name.clone();
                    track.sample = Some(resolved.clone());
                    track_sample(&name, &resolved)
                };
                out_lines.push(msg);
                i = j + 1;
            }
            s if s.starts_with('~') => {
                // Strip leading ~ and then collect any following tokens until a boundary.
                let first_part = s.trim_start_matches('~');
                let mut j = i + 1;
                let mut query_parts: Vec<String> = vec![first_part.to_string()];
                while j < tokens.len() && !is_boundary(tokens[j].as_str()) {
                    query_parts.push(tokens[j].clone());
                    j += 1;
                }
                let sample_query = query_parts.join(" ").trim().to_string();
                if sample_query.is_empty() {
                    return Err(anyhow!("usage: trackname ~ sample_query"));
                }
                let resolved = resolve_sample_path(&sample_query);
                let path = std::path::Path::new(&resolved);
                if !path.is_file() {
                    if path.exists() {
                        return Err(anyhow!("sample must be a file: {}", resolved));
                    }
                    let suggestions = find_similar_samples(&sample_query);
                    return Err(anyhow!("{}", not_found_sample(&sample_query, &suggestions)));
                }
                let msg = {
                    let (_, track) = track_mut(&mut draft, &track_name)?;
                    let name = track.name.clone();
                    track.sample = Some(resolved.clone());
                    track_sample(&name, &resolved)
                };
                out_lines.push(msg);
                i = j;
            }
            "mute" => {
                let msg = {
                    let (_, track) = track_mut(&mut draft, &track_name)?;
                    let name = track.name.clone();
                    track.mute = true;
                    track_muted(&name)
                };
                out_lines.push(msg);
                i += 1;
            }
            "unmute" => {
                let msg = {
                    let (_, track) = track_mut(&mut draft, &track_name)?;
                    let name = track.name.clone();
                    track.mute = false;
                    track_unmuted(&name)
                };
                out_lines.push(msg);
                i += 1;
            }
            "solo" => {
                let msg = {
                    let (_, track) = track_mut(&mut draft, &track_name)?;
                    let name = track.name.clone();
                    track.solo = !track.solo;
                    let is_solo = track.solo;
                    track_solo(&name, is_solo)
                };
                out_lines.push(msg);
                i += 1;
            }
            ">" => {
                let var_name = tokens
                    .get(i + 1)
                    .ok_or_else(|| anyhow!("usage: trackname > variation"))?
                    .to_string();
                let msg = {
                    let (_, track) = track_mut(&mut draft, &track_name)?;
                    let name = track.name.clone();
                    if var_name == "main" || var_name == "-" {
                        track.current_variation = None;
                        track_variation(&name, "main")
                    } else if track.variations.contains_key(&var_name) {
                        track.current_variation = Some(var_name.clone());
                        track_variation(&name, &var_name)
                    } else {
                        return Err(anyhow!("variation '{}' not found", var_name));
                    }
                };
                out_lines.push(msg);
                i += 2;
            }
            "delay" => {
                let action = tokens
                    .get(i + 1)
                    .ok_or_else(|| anyhow!("usage: trackname delay on|off|<time> [fb] [mix]"))?
                    .to_string();
                let msg = match action.as_str() {
                    "on" => {
                        let (_, track) = track_mut(&mut draft, &track_name)?;
                        let name = track.name.clone();
                        track.delay.on = true;
                        track_delay(&name, true, None, None, None)
                    }
                    "off" => {
                        let (_, track) = track_mut(&mut draft, &track_name)?;
                        let name = track.name.clone();
                        track.delay.on = false;
                        track_delay(&name, false, None, None, None)
                    }
                    time_val => {
                        // Optional numeric fb/mix args, stopping at the next boundary.
                        let mut j = i + 2;
                        let mut fb: Option<f32> = None;
                        let mut mix: Option<f32> = None;
                        if j < tokens.len() && !is_boundary(tokens[j].as_str()) {
                            fb = Some(parse_unit_range("feedback", &tokens[j])?);
                            j += 1;
                        }
                        if j < tokens.len() && !is_boundary(tokens[j].as_str()) {
                            mix = Some(parse_unit_range("mix", &tokens[j])?);
                            j += 1;
                        }

                        let msg = {
                            let (_, track) = track_mut(&mut draft, &track_name)?;
                            let name = track.name.clone();
                            track.delay.on = true;
                            track.delay.time = time_val.to_string();
                            if let Some(fb) = fb { track.delay.feedback = fb; }
                            if let Some(mix) = mix { track.delay.mix = mix; }
                            let t = track.delay.time.clone();
                            let fb = track.delay.feedback;
                            let mix = track.delay.mix;
                            track_delay(&name, true, Some(&t), Some(fb), Some(mix))
                        };

                        // We consumed: delay + time + (fb?) + (mix?)
                        i = j;
                        out_lines.push(msg);
                        continue;
                    }
                };
                out_lines.push(msg);
                i += 2;
            }
            "gen" => {
                let script = tokens
                    .get(i + 1)
                    .ok_or_else(|| anyhow!("usage: trackname gen euclid(5,16)"))?
                    .to_string();
                let script = script.trim_matches(|c| c == '`' || c == '"');
                let engine = PatternEngine::new();
                let pattern = engine.eval(script).map_err(|e| anyhow!("script error: {}", e))?;

                let msg = {
                    let (_, track) = track_mut(&mut draft, &track_name)?;
                    let name = track.name.clone();
                    if let Some(ref var_name) = variation {
                        track.variations.insert(var_name.clone(), Pattern::visual(&pattern));
                    } else {
                        track.pattern = Some(Pattern::visual(&pattern));
                    }
                    track_generated(&name, &pattern)
                };
                out_lines.push(msg);
                i += 2;
            }
            "ai" => {
                // Consume the rest of the line as the description.
                let description: String = tokens
                    .iter()
                    .skip(i + 1)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim_matches('"')
                    .to_string();
                if description.trim().is_empty() {
                    return Err(anyhow!("usage: trackname ai \"description\""));
                }

                let config = AiConfig::default();
                let context = PatternContext {
                    bpm: draft.bpm,
                    steps: 16,
                    target_track: track_name.clone(),
                    tracks: draft
                        .tracks
                        .iter()
                        .map(|t| TrackInfo {
                            name: t.name.clone(),
                            sample: t.sample.clone(),
                            pattern: t.active_pattern().map(|p| match p {
                                Pattern::Visual(s) => s.clone(),
                            }),
                            variations: t.variations.keys().cloned().collect(),
                            current_variation: t.current_variation.clone(),
                            muted: t.mute,
                            solo: t.solo,
                            gain_db: t.gain_db,
                            delay: if t.delay.on {
                                Some(DelayInfo {
                                    on: t.delay.on,
                                    time: t.delay.time.clone(),
                                    feedback: t.delay.feedback,
                                    mix: t.delay.mix,
                                })
                            } else {
                                None
                            },
                        })
                        .collect(),
                };

                match suggest_patterns(&config, &description, &context) {
                    Ok(patterns) => {
                        let Some(pattern) = patterns.first() else {
                            return Err(anyhow!("AI returned no valid patterns"));
                        };
                        let msg = {
                            let (_, track) = track_mut(&mut draft, &track_name)?;
                            let name = track.name.clone();
                            if let Some(ref var_name) = variation {
                                track.variations.insert(var_name.clone(), Pattern::visual(pattern));
                                track_pattern(&format!("{}.{}", name, var_name), pattern)
                            } else {
                                track.pattern = Some(Pattern::visual(pattern));
                                track_pattern(&name, pattern)
                            }
                        };
                        out_lines.push(msg);
                        break;
                    }
                    Err(e) => return Err(anyhow!("AI generation failed: {}", e)),
                }
            }
            other => {
                return Err(anyhow!("unknown track command segment: {}", other));
            }
        }
    }

    if out_lines.is_empty() {
        return Ok(None);
    }

    *song = draft;
    crate::audio::reload_song(song);
    Ok(Some(Output::Text(out_lines.join("\n"))))
}

/// Check if a string looks like a pattern (contains x, X, or . at start)
fn looks_like_pattern(s: &str) -> bool {
    let first = s.chars().next().unwrap_or(' ');
    matches!(first, 'x' | 'X' | '.')
}

/// Check if a string looks like a gain value (-3db, +2db, -3, +2)
fn looks_like_gain(s: &str) -> bool {
    if s.is_empty() { return false; }
    let first = s.chars().next().unwrap_or(' ');
    (first == '-' || first == '+') && s.len() > 1
}

/// Parse a gain value like "-3db", "+2db", "-3", "+2"
fn parse_gain_value(s: &str) -> Result<f32> {
    let cleaned = s.trim_end_matches("db").trim_end_matches("dB");
    cleaned.parse().map_err(|_| anyhow!("invalid gain value: {}", s))
}

fn parse_track_index(song: &Song, raw: &str) -> Result<usize> {
    // Try numeric index first (1-based)
    if let Ok(idx) = raw.parse::<usize>() {
        if idx == 0 || idx > song.tracks.len() {
            bail!("no such track index");
        }
        return Ok(idx - 1);
    }
    
    // Try name match (case-insensitive)
    let raw_lower = raw.to_lowercase();
    for (i, track) in song.tracks.iter().enumerate() {
        if track.name.to_lowercase() == raw_lower {
            return Ok(i);
        }
    }
    
    bail!("no track found: {}", raw);
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

// --- Track Names State (for completer access) ---
static TRACK_NAMES: once_cell::sync::Lazy<StdMutex<Vec<String>>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(Vec::new()));

/// Update the shared track names cache (called when song changes).
pub fn update_track_names(song: &Song) {
    if let Ok(mut names) = TRACK_NAMES.lock() {
        *names = song.tracks.iter().map(|t| t.name.clone()).collect();
    }
}

/// Get current track names for autocomplete.
pub fn get_track_names() -> Vec<String> {
    TRACK_NAMES.lock().map(|g| g.clone()).unwrap_or_default()
}

fn set_live_view(on: bool) {
    LIVE_VIEW.store(on, Ordering::SeqCst);
}

pub(crate) fn live_view_enabled() -> bool {
    LIVE_VIEW.load(Ordering::SeqCst)
}

// ANSI helpers for simple highlighting
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

// Sample path helpers

/// Resolve sample path shortcuts to full paths.
/// Supports:
/// - Full paths: "samples/kits/harsh 909/Kick.wav"
/// - Kit shortcuts: "909/kick" -> finds matching sample
/// - Name only: "kick" -> finds first matching sample
fn resolve_sample_path(input: &str) -> String {
    let raw = input.trim();
    if raw.is_empty() {
        return raw.to_string();
    }

    // Already a valid path
    if std::path::Path::new(raw).exists() {
        return raw.to_string();
    }

    // Token-based fuzzy match: all tokens must be present (any order).
    let tokens = sample_query_tokens(raw);
    if tokens.is_empty() {
        return raw.to_string();
    }

    let samples = scan_all_samples();
    let mut best: Option<(i32, String)> = None;
    for s in samples {
        let Some(score) = score_sample_candidate(&tokens, raw, &s) else { continue };
        match &best {
            None => best = Some((score, s)),
            Some((best_score, best_path)) => {
                let replace = score > *best_score
                    || (score == *best_score && s.len() < best_path.len())
                    || (score == *best_score && s.len() == best_path.len() && s < *best_path);
                if replace {
                    best = Some((score, s));
                }
            }
        }
    }

    best.map(|(_, s)| s).unwrap_or_else(|| raw.to_string())
}

/// Find samples similar to the given query for suggestions.
fn find_similar_samples(query: &str) -> Vec<String> {
    let raw = query.trim();
    if raw.is_empty() {
        return Vec::new();
    }

    let tokens = sample_query_tokens(raw);
    if tokens.is_empty() {
        return Vec::new();
    }

    let samples = scan_all_samples();
    let mut matches: Vec<(i32, String)> = samples
        .into_iter()
        .filter_map(|s| {
            let score = partial_similarity_score(&tokens, raw, &s);
            if score > 0 { Some((score, s)) } else { None }
        })
        .collect();

    matches.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.len().cmp(&b.1.len()))
            .then_with(|| a.1.cmp(&b.1))
    });
    matches.into_iter().map(|(_, s)| s).collect()
}

fn sample_query_tokens(query: &str) -> Vec<String> {
    query
        .split(|c: char| c.is_whitespace() || c == '/' || c == '\\')
        .filter_map(|t| {
            let t = t.trim().to_lowercase();
            if t.is_empty() { None } else { Some(t) }
        })
        .collect()
}

fn score_sample_candidate(tokens: &[String], raw_query: &str, path: &str) -> Option<i32> {
    let path_lower = path.to_lowercase();
    if !tokens.iter().all(|t| path_lower.contains(t)) {
        return None;
    }
    let filename = std::path::Path::new(path)
        .file_stem()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut score: i32 = 0;
    if tokens.len() == 1 && filename == tokens[0] {
        score += 100;
    }
    for t in tokens {
        score += if filename.contains(t) { 30 } else { 10 };
    }

    let raw_lower = raw_query.to_lowercase();
    if !raw_lower.is_empty() && path_lower.contains(&raw_lower) {
        score += 15;
    }
    Some(score.max(1))
}

fn partial_similarity_score(tokens: &[String], raw_query: &str, path: &str) -> i32 {
    let path_lower = path.to_lowercase();
    let filename = std::path::Path::new(path)
        .file_stem()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut hits: i32 = 0;
    let mut score: i32 = 0;
    for t in tokens {
        if path_lower.contains(t) {
            hits += 1;
            score += if filename.contains(t) { 20 } else { 5 };
        }
    }
    if hits == 0 {
        return 0;
    }
    let raw_lower = raw_query.to_lowercase();
    if !raw_lower.is_empty() && path_lower.contains(&raw_lower) {
        score += 10;
    }
    score + hits * 10
}

/// List available samples, optionally filtered.
fn list_available_samples(filter: Option<&str>) -> String {
    let samples = scan_all_samples();
    
    let filtered: Vec<&String> = if let Some(f) = filter {
        let f_lower = f.to_lowercase();
        samples.iter().filter(|s| s.to_lowercase().contains(&f_lower)).collect()
    } else {
        samples.iter().collect()
    };
    
    if filtered.is_empty() {
        return String::new();
    }
    
    // Group by directory
    let mut by_dir: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for s in filtered {
        let path = std::path::Path::new(s);
        let dir = path.parent()
            .and_then(|p| p.to_str())
            .unwrap_or("samples")
            .to_string();
        let filename = path.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(s)
            .to_string();
        by_dir.entry(dir).or_default().push(filename);
    }
    
    let mut output = String::new();
    for (dir, files) in by_dir {
        output.push_str(&format!("\n{}:\n", dir));
        for f in files {
            output.push_str(&format!("  {}\n", f));
        }
    }
    
    if let Some(f) = filter {
        output.insert_str(0, &format!("Samples matching '{}':", f));
    } else {
        output.insert_str(0, "Available samples:");
    }
    
    output.trim_end().to_string()
}

/// Scan all samples in the samples directory.
fn scan_all_samples() -> Vec<String> {
    let mut samples = Vec::new();
    let samples_dir = std::path::Path::new("samples");
    if samples_dir.is_dir() {
        collect_samples_recursive(samples_dir, &mut samples);
    }
    samples.sort();
    samples
}

fn collect_samples_recursive(dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_samples_recursive(&path, out);
        } else if is_audio_file(&path) {
            if let Some(s) = path.to_str() {
                out.push(s.to_string());
            }
        }
    }
}

fn is_audio_file(path: &std::path::Path) -> bool {
    let Some(ext) = path.extension() else { return false };
    let ext = ext.to_string_lossy().to_lowercase();
    matches!(ext.as_str(), "wav" | "mp3" | "ogg" | "flac" | "aiff" | "aif")
}

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
            ],
            global_step: 2,
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
            ],
            global_step: 0,
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
    fn parse_track_index_supports_track_name() {
        let mut song = Song::default();
        song.tracks.push(Track::new("Kick"));
        song.tracks.push(Track::new("Snare"));
        song.tracks.push(Track::new("Hi-Hat"));

        // Numeric indices still work (1-based)
        assert_eq!(parse_track_index(&song, "1").unwrap(), 0);
        assert_eq!(parse_track_index(&song, "2").unwrap(), 1);

        // Exact name match (case-insensitive)
        assert_eq!(parse_track_index(&song, "Kick").unwrap(), 0);
        assert_eq!(parse_track_index(&song, "kick").unwrap(), 0);
        assert_eq!(parse_track_index(&song, "SNARE").unwrap(), 1);
        assert_eq!(parse_track_index(&song, "Hi-Hat").unwrap(), 2);

        // Invalid names/indices return errors
        assert!(parse_track_index(&song, "Bass").is_err());
        assert!(parse_track_index(&song, "0").is_err());
        assert!(parse_track_index(&song, "99").is_err());
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
        // Use full path to avoid resolution changing it
        let output = handle_line(
            &mut song,
            "track(\"Kick\").sample(1, \"samples/kits/harsh 909/Kick.wav\").pattern(1, \"x...\")",
        )
        .expect("chain executes");

        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].name, "Kick");
        // Sample path may be resolved, just check it contains "Kick"
        assert!(song.tracks[0].sample.as_deref().unwrap().contains("Kick"));
        match &song.tracks[0].pattern {
            Some(Pattern::Visual(pat)) => assert_eq!(pat, "x..."),
            other => panic!("unexpected pattern state: {:?}", other),
        }

        if let Output::Text(t) = output {
            // New styled output uses ● for hits
            assert!(t.contains("●") || t.contains("x..."));
            assert!(t.contains("Kick") || t.contains("kick"));
        } else { panic!("expected text output"); }
    }

    #[test]
    fn multi_add_tracks_atomic_success() {
        let mut song = Song::default();
        handle_line(&mut song, "+ kick snare hat").expect("multi add");
        assert_eq!(song.tracks.len(), 3);
        assert_eq!(song.tracks[0].name, "kick");
        assert_eq!(song.tracks[1].name, "snare");
        assert_eq!(song.tracks[2].name, "hat");
    }

    #[test]
    fn multi_add_tracks_atomic_failure_adds_nothing() {
        let mut song = Song::default();
        song.tracks.push(Track::new("Kick"));

        let res = handle_line(&mut song, "+ kick snare");
        assert!(res.is_err(), "expected error for duplicate track name");
        assert_eq!(song.tracks.len(), 1, "multi-add should be atomic");
        assert_eq!(song.tracks[0].name, "Kick");
    }

    #[test]
    fn track_first_chaining_atomic_success() {
        let mut song = Song::default();
        song.tracks.push(Track::new("Kick"));

        handle_line(&mut song, "kick x... ~[harsh 909 kick] -3db").expect("chained track cmd");

        match &song.tracks[0].pattern {
            Some(Pattern::Visual(p)) => assert_eq!(p, "x..."),
            other => panic!("unexpected pattern state: {:?}", other),
        }
        assert!(
            song.tracks[0]
                .sample
                .as_deref()
                .unwrap_or("")
                .contains("Kick.wav"),
            "expected sample path to resolve to Kick.wav"
        );
        assert_eq!(song.tracks[0].gain_db, -3.0);
    }

    #[test]
    fn track_first_chaining_atomic_failure_rolls_back() {
        let mut song = Song::default();
        let mut track = Track::new("Kick");
        track.pattern = Some(Pattern::visual("...."));
        track.sample = Some("samples/kits/harsh 909/Kick.wav".into());
        track.gain_db = 0.0;
        song.tracks.push(track);

        let res = handle_line(&mut song, "kick x... ~[definitely no such sample] -3db");
        assert!(res.is_err(), "expected sample resolution to fail");

        // Atomic per-line: nothing should have changed
        match &song.tracks[0].pattern {
            Some(Pattern::Visual(p)) => assert_eq!(p, "...."),
            other => panic!("unexpected pattern state: {:?}", other),
        }
        assert_eq!(
            song.tracks[0].sample.as_deref(),
            Some("samples/kits/harsh 909/Kick.wav")
        );
        assert_eq!(song.tracks[0].gain_db, 0.0);
    }

    #[test]
    fn semicolon_separates_multiple_commands_in_one_line() {
        let mut song = Song::default();
        handle_line(&mut song, "+ kick snare; kick x...; snare ....x.......x...").expect("semicolon chain");
        assert_eq!(song.tracks.len(), 2);
        assert_eq!(song.tracks[0].name, "kick");
        assert_eq!(song.tracks[1].name, "snare");
        match &song.tracks[0].pattern {
            Some(Pattern::Visual(p)) => assert_eq!(p, "x..."),
            other => panic!("unexpected pattern state: {:?}", other),
        }
    }

    #[test]
    fn semicolon_chain_stops_on_first_error() {
        let mut song = Song::default();
        let res = handle_line(&mut song, "+ kick; definitely_not_a_command; + snare");
        assert!(res.is_err(), "expected unknown command error");
        // First command should have applied; later commands should not.
        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].name, "kick");
    }

    #[test]
    fn semicolon_does_not_split_inside_quotes() {
        let mut song = Song::default();
        handle_line(&mut song, "track(\"Kick\"); pattern 1 \"x...;x...\"").expect("semicolon + quoted pattern");
        assert_eq!(song.tracks.len(), 1);
        match &song.tracks[0].pattern {
            Some(Pattern::Visual(p)) => assert_eq!(p, "x...;x..."),
            other => panic!("unexpected pattern state: {:?}", other),
        }
    }

    #[test]
    fn sample_command_rejects_directory_path() {
        let mut song = Song::default();
        song.tracks.push(Track::new("Kick"));

        let res = handle_line(&mut song, "sample 1 documentation");
        assert!(res.is_err(), "directories must not be accepted as samples");
        assert!(song.tracks[0].sample.is_none(), "track sample must remain unchanged");
    }

    #[test]
    fn track_first_sample_rejects_directory_path() {
        let mut song = Song::default();
        song.tracks.push(Track::new("Kick"));

        let res = handle_line(&mut song, "kick ~ documentation");
        assert!(res.is_err(), "directories must not be accepted as samples");
        assert!(song.tracks[0].sample.is_none(), "track sample must remain unchanged");
    }
}
