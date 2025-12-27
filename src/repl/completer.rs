//! Tab completion for the groove-cli REPL.

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow;
use std::path::Path;

/// REPL helper providing command and sample path completion.
pub struct GrooveHelper {
    /// Cached sample paths for fast lookup
    sample_cache: Vec<String>,
}

impl GrooveHelper {
    pub fn new() -> Self {
        let sample_cache = scan_samples_dir();
        Self { sample_cache }
    }
    
    /// Refresh the sample cache (call when samples dir changes).
    #[allow(dead_code)]
    pub fn refresh_samples(&mut self) {
        self.sample_cache = scan_samples_dir();
    }
}

/// Scan the samples directory recursively and return all sample paths.
fn scan_samples_dir() -> Vec<String> {
    let mut samples = Vec::new();
    let samples_dir = Path::new("samples");
    if samples_dir.is_dir() {
        collect_samples(samples_dir, &mut samples);
    }
    samples.sort();
    samples
}

fn collect_samples(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_samples(&path, out);
        } else if is_audio_file(&path) {
            if let Some(s) = path.to_str() {
                out.push(s.to_string());
            }
        }
    }
}

fn is_audio_file(path: &Path) -> bool {
    let Some(ext) = path.extension() else { return false };
    let ext = ext.to_string_lossy().to_lowercase();
    matches!(ext.as_str(), "wav" | "mp3" | "ogg" | "flac" | "aiff" | "aif")
}

/// Commands available in the REPL.
const COMMANDS: &[&str] = &[
    "bpm", "steps", "swing", "track", "pattern", "sample", "samples",
    "delay", "mute", "solo", "gain", "playback", "div", "preview",
    "remove", "list", "play", "stop", "save", "open", "clear",
    "gen", "var", "ai",
];

/// Meta commands (prefixed with :).
const META_COMMANDS: &[&str] = &[
    ":help", ":q", ":quit", ":exit", ":doc",
    ":live", ":live on", ":live off",
];

impl Completer for GrooveHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_to_pos = &line[..pos];
        
        // Meta commands
        if line_to_pos.starts_with(':') {
            let matches: Vec<Pair> = META_COMMANDS
                .iter()
                .filter(|cmd| cmd.starts_with(line_to_pos))
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                })
                .collect();
            return Ok((0, matches));
        }
        
        // Check if we're completing a sample path (inside quotes after "sample")
        if let Some(sample_start) = find_sample_completion_start(line_to_pos) {
            let prefix = &line_to_pos[sample_start..];
            let matches: Vec<Pair> = self.sample_cache
                .iter()
                .filter(|path| path.starts_with(prefix) || path.contains(prefix))
                .map(|path| Pair {
                    display: path.clone(),
                    replacement: format!("\"{}\"", path),
                })
                .take(20) // Limit results
                .collect();
            return Ok((sample_start, matches));
        }
        
        // Command completion at the start of line
        let words: Vec<&str> = line_to_pos.split_whitespace().collect();
        if words.is_empty() || (words.len() == 1 && !line_to_pos.ends_with(' ')) {
            let prefix = words.first().copied().unwrap_or("");
            let matches: Vec<Pair> = COMMANDS
                .iter()
                .filter(|cmd| cmd.starts_with(prefix))
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                })
                .collect();
            return Ok((0, matches));
        }
        
        Ok((pos, Vec::new()))
    }
}

/// Find the start position of a sample path to complete.
/// Returns Some(pos) if we're inside a sample command's path argument.
fn find_sample_completion_start(line: &str) -> Option<usize> {
    // Look for patterns like: sample 1 "path or sample 1 path
    let lower = line.to_lowercase();
    
    // Check for sample command
    if !lower.contains("sample") {
        return None;
    }
    
    // Find the last quote or the start of an unquoted path
    if let Some(quote_pos) = line.rfind('"') {
        // Check if this is an opening quote (no closing quote after it)
        let after_quote = &line[quote_pos + 1..];
        if !after_quote.contains('"') {
            return Some(quote_pos + 1);
        }
    }
    
    // Check for unquoted path after sample command
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 && parts[0].to_lowercase() == "sample" {
        // If we have "sample <idx> <partial_path>", complete the path
        if parts.len() >= 3 {
            let path_start = line.rfind(parts[parts.len() - 1]).unwrap_or(0);
            // Only complete if it looks like a path
            if parts[parts.len() - 1].contains('/') || parts[parts.len() - 1].starts_with("samples") {
                return Some(path_start);
            }
        }
    }
    
    None
}

impl Hinter for GrooveHelper {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        // Could add inline hints here later
        None
    }
}

impl Highlighter for GrooveHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // Dim the hint
        Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }
}

impl Validator for GrooveHelper {}

impl Helper for GrooveHelper {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_samples_finds_audio_files() {
        // This test will pass if samples dir exists, otherwise returns empty
        let samples = scan_samples_dir();
        // Just check it doesn't panic
        assert!(samples.is_empty() || samples.iter().all(|s| is_audio_file(Path::new(s))));
    }

    #[test]
    fn is_audio_file_recognizes_formats() {
        assert!(is_audio_file(Path::new("test.wav")));
        assert!(is_audio_file(Path::new("test.mp3")));
        assert!(is_audio_file(Path::new("test.WAV")));
        assert!(!is_audio_file(Path::new("test.txt")));
        assert!(!is_audio_file(Path::new("test")));
    }

    #[test]
    fn find_sample_completion_detects_sample_command() {
        assert!(find_sample_completion_start(r#"sample 1 "samples/"#).is_some());
        assert!(find_sample_completion_start(r#"sample 1 samples/k"#).is_some());
        assert!(find_sample_completion_start("bpm 120").is_none());
    }
}

