//! Tab completion for the groove-cli REPL.

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow;
use std::path::Path;

use super::get_track_names;

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

    /// Fuzzy match samples against a query, returning scored results.
    /// If query is empty, returns all samples.
    fn fuzzy_match_samples(&self, query: &str) -> Vec<(i32, String)> {
        // Empty query: return all samples
        if query.is_empty() {
            return self.sample_cache
                .iter()
                .map(|p| (50, p.clone()))
                .collect();
        }
        
        let query_lower = query.to_lowercase();
        let query_parts: Vec<&str> = query_lower.split('/').collect();
        
        let mut scored: Vec<(i32, String)> = self.sample_cache
            .iter()
            .filter_map(|path| {
                let path_lower = path.to_lowercase();
                let filename = Path::new(path)
                    .file_stem()
                    .and_then(|f| f.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                
                // Score based on match quality - only good matches
                let score = if filename == query_lower {
                    100 // Exact filename match
                } else if filename.starts_with(&query_lower) {
                    80 // Filename starts with query
                } else if filename.contains(&query_lower) {
                    60 // Filename contains query
                } else if path_lower.contains(&query_lower) {
                    40 // Path contains query
                } else if query_parts.len() > 1 {
                    // Multi-part query like "909/kick" - check if all parts match in order
                    let mut search_pos = 0;
                    let mut all_match = true;
                    for part in &query_parts {
                        if let Some(pos) = path_lower[search_pos..].find(part) {
                            search_pos += pos + part.len();
                        } else {
                            all_match = false;
                            break;
                        }
                    }
                    if all_match { 50 } else { 0 }
                } else {
                    // No fuzzy single-char matching - too noisy
                    // Only match if query is a substring
                    0
                };
                
                if score > 0 {
                    Some((score, path.clone()))
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by score descending
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored
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

/// Commands available in the REPL (old + new style).
const COMMANDS: &[&str] = &[
    // New style
    "go", "stop", "list", "ls", "save", "open",
    // Old style (still work)
    "bpm", "steps", "swing", "track", "pattern", "sample", "samples", "browse",
    "delay", "mute", "solo", "gain", "playback", "div", "preview",
    "remove", "list", "play", "stop", "save", "open", "clear",
    "gen", "var", "ai",
];

/// Meta commands (prefixed with :).
const META_COMMANDS: &[&str] = &[
    ":help", ":q", ":quit", ":exit", ":doc",
    ":live", ":live on", ":live off",
    "?",
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
        
        // Meta commands and help
        if line_to_pos.starts_with(':') || line_to_pos == "?" {
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

        // Check for new-style sample completion: "trackname ~ query"
        if let Some((start, query)) = find_tilde_sample_context(line_to_pos) {
            let matches: Vec<Pair> = self.fuzzy_match_samples(&query)
                .into_iter()
                .take(15)
                .map(|(_, path)| {
                    // Extract just the useful part (kit/sample)
                    let display = shorten_sample_path(&path);
                    Pair {
                        display,
                        // Include space before path for proper syntax: "kick ~ samples/..."
                        replacement: format!(" {}", path),
                    }
                })
                .collect();
            // Start replacement from the ~ character to include proper spacing
            let tilde_pos = line_to_pos.find('~').unwrap_or(start);
            return Ok((tilde_pos + 1, matches));
        }
        
        // Check for old-style sample completion (inside quotes after "sample")
        if let Some((sample_start, inside_quotes)) = find_sample_completion_start(line_to_pos) {
            let prefix = &line_to_pos[sample_start..];
            let matches: Vec<Pair> = self.fuzzy_match_samples(prefix)
                .into_iter()
                .take(15)
                .map(|(_, path)| Pair {
                    display: shorten_sample_path(&path),
                    replacement: if inside_quotes {
                        format!("{}\"", path)
                    } else {
                        format!("\"{}\"", path)
                    },
                })
                .collect();
            return Ok((sample_start, matches));
        }

        // Check for new-style track-first commands: "trackname ..."
        // First word could be a track name if it's in our track list
        let words: Vec<&str> = line_to_pos.split_whitespace().collect();
        let track_names = get_track_names();
        
        if !words.is_empty() {
            let first = words[0].to_lowercase();
            let is_track = track_names.iter().any(|t| t.to_lowercase() == first);
            
            // If first word is a track and we're typing the second word
            if is_track && words.len() == 1 && line_to_pos.ends_with(' ') {
                // Suggest track subcommands
                let subcommands = vec!["mute", "unmute", "solo", "delay", "gen", "ai", ">", "~"];
                let matches: Vec<Pair> = subcommands
                    .into_iter()
                    .map(|cmd| Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    })
                    .collect();
                return Ok((pos, matches));
            }
        }

        // Check if we're completing a track name (for old-style commands)
        if let Some((start, prefix)) = find_track_completion_context(line_to_pos) {
            let prefix_lower = prefix.to_lowercase();
            let matches: Vec<Pair> = track_names
                .into_iter()
                .filter(|name| {
                    prefix.is_empty() || name.to_lowercase().starts_with(&prefix_lower)
                })
                .map(|name| Pair {
                    display: name.clone(),
                    replacement: name,
                })
                .collect();
            return Ok((start, matches));
        }
        
        // Command completion at the start of line
        if words.is_empty() || (words.len() == 1 && !line_to_pos.ends_with(' ')) {
            let prefix = words.first().copied().unwrap_or("");
            let prefix_lower = prefix.to_lowercase();
            
            // Include both commands and track names
            let mut matches: Vec<Pair> = COMMANDS
                .iter()
                .filter(|cmd| cmd.starts_with(&prefix_lower))
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                })
                .collect();
            
            // Also suggest track names (for track-first syntax)
            let track_matches: Vec<Pair> = track_names
                .into_iter()
                .filter(|name| name.to_lowercase().starts_with(&prefix_lower))
                .map(|name| Pair {
                    display: format!("{} (track)", name),
                    replacement: name,
                })
                .collect();
            matches.extend(track_matches);
            
            // Special: + for adding track
            if "+".starts_with(&prefix_lower) {
                matches.push(Pair {
                    display: "+ (add track)".to_string(),
                    replacement: "+ ".to_string(),
                });
            }
            
            return Ok((0, matches));
        }
        
        Ok((pos, Vec::new()))
    }
}

/// Shorten a sample path for display (show kit/sample instead of full path)
fn shorten_sample_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        // Show last two parts: "kit/Sample.wav"
        parts[parts.len()-2..].join("/")
    } else {
        path.to_string()
    }
}

/// Check for "trackname ~ query" pattern
fn find_tilde_sample_context(line: &str) -> Option<(usize, String)> {
    if let Some(tilde_pos) = line.find('~') {
        let after_tilde = &line[tilde_pos + 1..];
        let query = after_tilde.trim_start();
        let query_start = tilde_pos + 1 + (after_tilde.len() - query.len());
        return Some((query_start, query.to_string()));
    }
    None
}

/// Commands that take a track identifier (index or name) as first argument.
const TRACK_COMMANDS: &[&str] = &[
    "pattern", "sample", "mute", "solo", "gain", "playback", "div",
    "delay", "var", "remove", "gen", "browse", "unmute",
];

/// Check if we're in a context where track name completion should be offered.
/// Returns Some((start_pos, prefix)) if completing a track name.
fn find_track_completion_context(line: &str) -> Option<(usize, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    
    let cmd = parts[0].to_lowercase();
    if !TRACK_COMMANDS.contains(&cmd.as_str()) {
        return None;
    }
    
    // If command only (e.g., "mute "), we're completing the track
    if parts.len() == 1 && line.ends_with(' ') {
        return Some((line.len(), String::new()));
    }
    
    // If we have command + partial track (e.g., "mute Ki"), complete the partial
    if parts.len() == 2 && !line.ends_with(' ') {
        let prefix = parts[1].to_string();
        let start = line.rfind(parts[1]).unwrap_or(0);
        return Some((start, prefix));
    }
    
    None
}

/// Find the start position of a sample path to complete.
/// Returns Some((pos, inside_quotes)) if we're inside a sample command's path argument.
fn find_sample_completion_start(line: &str) -> Option<(usize, bool)> {
    // Look for patterns like: sample 1 "path or sample 1 path
    let lower = line.to_lowercase();
    
    // Check for sample or preview command
    if !lower.contains("sample") && !lower.contains("preview") {
        return None;
    }
    
    // Find the last quote or the start of an unquoted path
    if let Some(quote_pos) = line.rfind('"') {
        // Check if this is an opening quote (no closing quote after it)
        let after_quote = &line[quote_pos + 1..];
        if !after_quote.contains('"') {
            // We're inside quotes - return position after quote
            return Some((quote_pos + 1, true));
        }
    }
    
    // Check for unquoted path after sample command
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 && (parts[0].to_lowercase() == "sample" || parts[0].to_lowercase() == "preview") {
        // If we have "sample <idx> <partial_path>", complete the path
        if parts.len() >= 3 || (parts[0].to_lowercase() == "preview" && parts.len() >= 2) {
            let last_part = parts[parts.len() - 1];
            let path_start = line.rfind(last_part).unwrap_or(0);
            // Only complete if it looks like a path
            if last_part.contains('/') || last_part.starts_with("samples") || !last_part.is_empty() {
                return Some((path_start, false));
            }
        }
    }
    
    None
}

impl Hinter for GrooveHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        // Show inline hint for ~ sample completion
        if let Some((_, query)) = find_tilde_sample_context(&line[..pos]) {
            if query.is_empty() {
                // Show hint that Tab will complete
                if !self.sample_cache.is_empty() {
                    return Some(" (Tab for samples)".to_string());
                }
            } else {
                let matches = self.fuzzy_match_samples(&query);
                if matches.is_empty() {
                    return Some(" (no matches)".to_string());
                } else if let Some((_, top_match)) = matches.first() {
                    // Show the rest of the top match as a hint
                    let short = shorten_sample_path(top_match);
                    if short.to_lowercase().starts_with(&query.to_lowercase()) {
                        return Some(short[query.len()..].to_string());
                    } else {
                        // Show full suggestion if it doesn't start with query
                        return Some(format!(" → {}", short));
                    }
                }
            }
        }
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

    #[test]
    fn find_track_completion_detects_track_commands() {
        // Commands that expect track as first arg
        assert!(find_track_completion_context("pattern ").is_some());
        assert!(find_track_completion_context("mute ").is_some());
        assert!(find_track_completion_context("solo ").is_some());
        assert!(find_track_completion_context("sample ").is_some());
        assert!(find_track_completion_context("gain ").is_some());
        assert!(find_track_completion_context("delay ").is_some());
        assert!(find_track_completion_context("var ").is_some());
        assert!(find_track_completion_context("remove ").is_some());
        
        // With partial input
        assert!(find_track_completion_context("mute Ki").is_some());
        
        // Not a track command
        assert!(find_track_completion_context("bpm ").is_none());
        assert!(find_track_completion_context("track ").is_none()); // track creates, not selects
    }

    #[test]
    fn find_tilde_sample_context_works() {
        assert_eq!(find_tilde_sample_context("kick ~ "), Some((7, "".to_string())));
        assert_eq!(find_tilde_sample_context("kick ~ 909"), Some((7, "909".to_string())));
        assert_eq!(find_tilde_sample_context("kick ~ 909/kick"), Some((7, "909/kick".to_string())));
        assert!(find_tilde_sample_context("kick mute").is_none());
        // Also works without space after ~
        assert_eq!(find_tilde_sample_context("kick ~"), Some((6, "".to_string())));
        // Works with partial query
        assert_eq!(find_tilde_sample_context("kick ~k"), Some((6, "k".to_string())));
    }

    #[test]
    fn complete_after_tilde_returns_samples() {
        let helper = GrooveHelper {
            sample_cache: vec![
                "samples/kits/909/Kick.wav".to_string(),
                "samples/kits/909/Snare.wav".to_string(),
            ],
        };
        
        // Simulating Tab after "kick ~ "
        let (pos, matches) = helper.complete("kick ~ ", 7, &rustyline::Context::new(&rustyline::history::DefaultHistory::new()))
            .expect("complete should succeed");
        
        // Should return samples at position after ~ (pos 6 = right after ~)
        assert_eq!(pos, 6);
        assert_eq!(matches.len(), 2);
        // Replacement should include leading space
        assert!(matches[0].replacement.starts_with(" "));
    }

    #[test]
    fn fuzzy_match_scores_correctly() {
        let helper = GrooveHelper {
            sample_cache: vec![
                "samples/kits/909/Kick.wav".to_string(),
                "samples/kits/909/Snare.wav".to_string(),
                "samples/kits/808/Kick.wav".to_string(),
            ],
        };
        
        let matches = helper.fuzzy_match_samples("kick");
        assert!(!matches.is_empty());
        // Should find both kicks
        assert!(matches.iter().any(|(_, p)| p.contains("909/Kick")));
        assert!(matches.iter().any(|(_, p)| p.contains("808/Kick")));
        
        // "909/kick" should match 909 kit specifically
        let matches909 = helper.fuzzy_match_samples("909/kick");
        assert!(!matches909.is_empty());
        assert!(matches909[0].1.contains("909"));
    }

    #[test]
    fn fuzzy_match_empty_query_returns_all() {
        let helper = GrooveHelper {
            sample_cache: vec![
                "samples/kits/909/Kick.wav".to_string(),
                "samples/kits/909/Snare.wav".to_string(),
            ],
        };
        
        // Empty query should return all samples
        let matches = helper.fuzzy_match_samples("");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn shorten_sample_path_works() {
        assert_eq!(
            shorten_sample_path("samples/kits/harsh 909/Kick.wav"),
            "harsh 909/Kick.wav"
        );
        assert_eq!(shorten_sample_path("Kick.wav"), "Kick.wav");
    }
}
