//! AI-powered pattern generation using LLMs.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for the AI pattern generator.
#[derive(Clone)]
pub struct AiConfig {
    /// OpenAI API key
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        // Try to load from .env file (check both locations)
        let _ = dotenvy::from_filename(".env");
        let _ = dotenvy::from_filename("src/.env");
        
        Self {
            api_key: env::var("OPENAI_API_KEY").ok(),
            model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.2".to_string()),
        }
    }
}

/// OpenAI Responses API request
#[derive(Serialize)]
struct ResponsesRequest {
    model: String,
    input: String,
    instructions: String,
    max_output_tokens: u32,
    store: bool,
}

/// OpenAI Responses API response
#[derive(Deserialize)]
struct ResponsesResponse {
    output: Vec<OutputItem>,
}

#[derive(Deserialize)]
struct OutputItem {
    content: Option<Vec<ContentItem>>,
}

#[derive(Deserialize)]
struct ContentItem {
    text: Option<String>,
}

/// Delay FX info for AI context
pub struct DelayInfo {
    pub on: bool,
    pub time: String,
    pub feedback: f32,
    pub mix: f32,
}

/// Track info for AI context
pub struct TrackInfo {
    pub name: String,
    pub sample: Option<String>,
    pub pattern: Option<String>,
    pub variations: Vec<String>,        // names of available variations
    pub current_variation: Option<String>,
    pub muted: bool,
    pub solo: bool,
    pub gain_db: f32,
    pub delay: Option<DelayInfo>,
}

/// Context for pattern generation (full song state).
pub struct PatternContext {
    pub bpm: u32,
    pub steps: usize,
    pub target_track: String,
    pub tracks: Vec<TrackInfo>,
}

impl Default for PatternContext {
    fn default() -> Self {
        Self {
            bpm: 120,
            steps: 16,
            target_track: String::new(),
            tracks: Vec::new(),
        }
    }
}

/// Check if a track name suggests a melodic instrument
fn is_melodic_track(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("synth") 
        || lower.contains("lead") 
        || lower.contains("melody")
        || lower.contains("bass")
        || lower.contains("arp")
        || lower.contains("pad")
        || lower.contains("keys")
        || lower.contains("piano")
}

/// Check if a pattern has pitch modifiers
fn has_pitch_modifiers(pattern: &str) -> bool {
    pattern.contains('+') || pattern.contains('-')
}

/// Generate pattern suggestions based on a description.
pub fn suggest_patterns(config: &AiConfig, description: &str, context: &PatternContext) -> Result<Vec<String>> {
    let is_melodic = is_melodic_track(&context.target_track);

    // For common drum-style prompts, return a deterministic pattern without calling the AI.
    // This keeps tests hermetic and avoids requiring an API key for simple cases.
    if !is_melodic {
        if let Some(pat) = keyword_pattern(description, context.steps) {
            return Ok(vec![pat]);
        }
    }

    let api_key = config.api_key.as_ref()
        .ok_or_else(|| anyhow!("OPENAI_API_KEY not set in .env"))?;

    let instructions = build_instructions(context);
    let input = build_input(description, context);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    // Try up to 3 times for melodic tracks if AI doesn't use pitch modifiers
    let max_attempts = if is_melodic { 3 } else { 1 };
    
    for attempt in 0..max_attempts {
        let request = ResponsesRequest {
            model: config.model.clone(),
            input: if attempt > 0 {
                format!("{}\n\nPREVIOUS ATTEMPT FAILED: You did not use pitch modifiers. You MUST use x+N or x-N syntax for melodic patterns. Try again.", input)
            } else {
                input.clone()
            },
            instructions: instructions.clone(),
            max_output_tokens: 200, // Increased for longer melodic patterns
            store: false,
        };
        
        let response = client
            .post("https://api.openai.com/v1/responses")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| anyhow!("AI request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("OpenAI API error {}: {}", status, body));
        }
        
        let result: ResponsesResponse = response.json()
            .map_err(|e| anyhow!("Failed to parse OpenAI response: {}", e))?;
        
        let content = result.output.first()
            .and_then(|o| o.content.as_ref())
            .and_then(|c| c.first())
            .and_then(|c| c.text.clone())
            .unwrap_or_default();
        
        let patterns = extract_patterns(&content);
        
        if patterns.is_empty() {
            continue;
        }
        
        // For melodic tracks, require pitch modifiers
        if is_melodic {
            let melodic_patterns: Vec<String> = patterns.into_iter()
                .filter(|p| has_pitch_modifiers(p))
                .collect();
            
            if !melodic_patterns.is_empty() {
                return Ok(melodic_patterns);
            }
            // Continue to next attempt
        } else {
            return Ok(patterns);
        }
    }
    
    Err(anyhow!("AI failed to generate melodic pattern with pitch modifiers after {} attempts. The AI keeps returning rhythm-only patterns.", max_attempts))
}

/// Build the system instructions with full context about what we're doing.
fn build_instructions(context: &PatternContext) -> String {
    format!(r#"You generate patterns for groove-cli (a step sequencer). Output ONLY the pattern string.

PATTERN SYNTAX (each character = 1 step):
- x = trigger at base pitch
- . = rest  
- x+N = trigger transposed up N semitones (e.g. x+7 = fifth, x+12 = octave)
- x-N = trigger transposed down N semitones

CRITICAL: For synth/lead/melody/bass tracks, you MUST use pitch modifiers (x+N, x-N) to create actual melodies and progressions. Plain "x" only patterns are WRONG for melodic instruments.

EXAMPLES:
Arpeggio: x...x+3...x+7...x+12...|x+12...x+7...x+3...x...
Synthwave: x+7..x+12.x...x+5..|x+3..x+7..x...x+10..
Bass line: x...x+5..x+7.x...|x-5...x..x+3..x+5..
Chord prog: x...x+4.x+7...|x+5..x+9.x+12..|x+3..x+7.x+10..|x...x+4.x+7...

BPM: {} | Steps: {}

Reply with ONLY the pattern. No explanation."#,
        context.bpm, context.steps
    )
}

/// Build the user input/prompt with current arrangement and request.
fn build_input(description: &str, context: &PatternContext) -> String {
    let mut input = String::new();
    
    // Show current arrangement if tracks exist
    if !context.tracks.is_empty() {
        input.push_str("Current song arrangement:\n\n");
        for t in &context.tracks {
            // Track header with name and sample
            let sample = t.sample.as_ref()
                .and_then(|s| s.split('/').last())
                .unwrap_or("(no sample)");
            input.push_str(&format!("Track: {} (sample: {})\n", t.name, sample));
            
            // Status flags
            let mut flags: Vec<String> = Vec::new();
            if t.muted { flags.push("muted".to_string()); }
            if t.solo { flags.push("solo".to_string()); }
            if t.gain_db != 0.0 { flags.push(format!("gain: {:.1}dB", t.gain_db)); }
            if !flags.is_empty() {
                input.push_str(&format!("  Status: {}\n", flags.join(", ")));
            }
            
            // Pattern
            if let Some(ref pat) = t.pattern {
                let var_info = t.current_variation.as_ref()
                    .map(|v| format!(" (playing: {})", v))
                    .unwrap_or_default();
                input.push_str(&format!("  Pattern: {}{}\n", pat, var_info));
            } else {
                input.push_str("  Pattern: (none)\n");
            }
            
            // Variations if any
            if !t.variations.is_empty() {
                input.push_str(&format!("  Variations: {}\n", t.variations.join(", ")));
            }
            
            // Delay FX
            if let Some(ref delay) = t.delay {
                if delay.on {
                    input.push_str(&format!(
                        "  Delay: {} time, {:.0}% feedback, {:.0}% mix\n",
                        delay.time, delay.feedback * 100.0, delay.mix * 100.0
                    ));
                }
            }
            
            input.push('\n');
        }
    }
    
    // The actual request
    let track_lower = context.target_track.to_lowercase();
    let is_melodic = track_lower.contains("synth") 
        || track_lower.contains("lead") 
        || track_lower.contains("melody")
        || track_lower.contains("bass")
        || track_lower.contains("arp")
        || track_lower.contains("pad");
    
    let melodic_hint = if is_melodic {
        "\n\nIMPORTANT: This is a melodic instrument. You MUST use pitch modifiers (x+N, x-N) to create an actual melody/progression. Do NOT output plain x patterns."
    } else {
        ""
    };
    
    input.push_str(&format!(
        "Create a {}-step pattern for track \"{}\".\nRequest: {}{}",
        context.steps, context.target_track, description, melodic_hint
    ));
    
    input
}

/// Match common descriptions to reliable patterns.
fn keyword_pattern(desc: &str, steps: usize) -> Option<String> {
    let d = desc.to_lowercase();
    
    // Order matters! More specific patterns first
    let pattern = if d.contains("one") && d.contains("hit") || d.contains("single") {
        "x..............."
    } else if d.contains("backbeat") || (d.contains("snare") && d.contains("2") && d.contains("4")) {
        "....x.......x..."  // Snare on beats 2 and 4
    } else if d.contains("offbeat") {
        "..x...x...x...x."
    } else if d.contains("4") && d.contains("floor") {
        "x...x...x...x..."  // Four on the floor (kick)
    } else if d.contains("simple") && d.contains("kick") {
        "x...x...x...x..."
    } else if d.contains("8th") || d.contains("eighth") {
        "x.x.x.x.x.x.x.x."
    } else if d.contains("16th") || d.contains("sixteenth") {
        "xxxxxxxxxxxxxxxx"
    } else if d.contains("sparse") {
        "x.......x......."
    } else if d.contains("syncopat") {
        "x..x..x...x..x.."
    } else if d.contains("shuffle") {
        "x..x..x..x..x..x"
    } else if d.contains("funk") {
        "x..x.x..x..x.x.."
    } else if d.contains("reggae") || d.contains("dub") {
        "..x...x...x...x."
    } else if d.contains("techno") && d.contains("kick") {
        "x...x...x...x..."
    } else if d.contains("dnb") || d.contains("drum and bass") || d.contains("jungle") {
        "x.....x.x......."
    } else if d.contains("trap") {
        "x..x..x.x..x..x."
    } else if d.contains("house") {
        "x...x...x...x..."
    } else {
        return None; // No match, use AI
    };
    
    // Adjust to requested step count
    Some(adjust_pattern_length(pattern, steps))
}

/// Repeat or truncate pattern to match target length.
fn adjust_pattern_length(pattern: &str, steps: usize) -> String {
    if pattern.len() == steps {
        pattern.to_string()
    } else if pattern.len() > steps {
        pattern[..steps].to_string()
    } else {
        pattern.chars().cycle().take(steps).collect()
    }
}

/// Extract valid patterns from LLM response.
fn extract_patterns(response: &str) -> Vec<String> {
    response
        .lines()
        .map(|line| {
            // Clean the line: trim whitespace, remove quotes/backticks
            line.trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim_matches('`')
                .trim()
        })
        .filter(|line| !line.is_empty())
        .filter(|line| is_valid_pattern(line))
        .map(|s| s.to_string())
        .take(3)
        .collect()
}

/// Check if a line looks like a valid pattern.
fn is_valid_pattern(s: &str) -> bool {
    // Must be reasonable length (4-128 chars, no spaces)
    if s.len() < 4 || s.len() > 128 || s.contains(' ') {
        return false;
    }
    // Must contain at least one hit (x/X)
    let has_hits = s.chars().any(|c| matches!(c, 'x' | 'X'));
    if !has_hits {
        return false;
    }
    // All chars must be valid pattern syntax (no spaces)
    // Include digits and +/- for pitch modifiers like x+7, x-3
    s.chars().all(|c| matches!(c, 'x' | 'X' | '.' | '_' | '|' | '^' | '~' | '(' | ')' | '+' | '-' | '0'..='9'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_patterns_filters_valid() {
        let response = r#"
Here are some patterns:
x...x...x...x...
This is not a pattern
x.x.x.x.x.x.x.x.
123456
X...x...X...x...
"#;
        let patterns = extract_patterns(response);
        assert_eq!(patterns.len(), 3);
        assert_eq!(patterns[0], "x...x...x...x...");
        assert_eq!(patterns[1], "x.x.x.x.x.x.x.x.");
        assert_eq!(patterns[2], "X...x...X...x...");
    }

    #[test]
    fn extract_patterns_rejects_spaces() {
        let response = "x... x... x... x...";
        let patterns = extract_patterns(response);
        assert!(patterns.is_empty());
    }

    #[test]
    fn build_instructions_includes_context() {
        let ctx = PatternContext {
            bpm: 140,
            steps: 16,
            target_track: "kick".to_string(),
            tracks: vec![],
        };
        let instructions = build_instructions(&ctx);
        assert!(instructions.contains("140"));
        assert!(instructions.contains("16"));
        assert!(instructions.contains("groove-cli"));
        assert!(instructions.contains("PATTERN SYNTAX"));
    }

    #[test]
    fn build_input_includes_tracks() {
        let ctx = PatternContext {
            bpm: 120,
            steps: 16,
            target_track: "snare".to_string(),
            tracks: vec![
                TrackInfo {
                    name: "kick".to_string(),
                    sample: Some("samples/909/kick.wav".to_string()),
                    pattern: Some("x...x...x...x...".to_string()),
                    variations: vec!["a".to_string(), "b".to_string()],
                    current_variation: None,
                    muted: false,
                    solo: false,
                    gain_db: 0.0,
                    delay: Some(DelayInfo { on: true, time: "1/4".to_string(), feedback: 0.3, mix: 0.25 }),
                },
            ],
        };
        let input = build_input("punchy backbeat", &ctx);
        assert!(input.contains("kick"));
        assert!(input.contains("x...x...x...x..."));
        assert!(input.contains("snare"));
        assert!(input.contains("punchy backbeat"));
        // Check new fields are in the output
        assert!(input.contains("Variations: a, b"));
        assert!(input.contains("Delay:"));
    }

    #[test]
    fn keyword_patterns_work() {
        assert_eq!(keyword_pattern("simple kick", 16), Some("x...x...x...x...".to_string()));
        assert_eq!(keyword_pattern("4 on the floor", 16), Some("x...x...x...x...".to_string()));
        assert_eq!(keyword_pattern("offbeat hat", 16), Some("..x...x...x...x.".to_string()));
        assert_eq!(keyword_pattern("backbeat snare", 16), Some("....x.......x...".to_string()));
        assert_eq!(keyword_pattern("random weird thing", 16), None); // falls back to AI
    }

    #[test]
    fn suggest_patterns_uses_keyword_patterns_without_api_key_for_drums() {
        let cfg = AiConfig { api_key: None, model: "gpt-5.2".to_string() };
        let ctx = PatternContext {
            bpm: 120,
            steps: 16,
            target_track: "kick".to_string(),
            tracks: vec![],
        };

        let patterns = suggest_patterns(&cfg, "4 on the floor", &ctx).expect("keyword match should succeed");
        assert_eq!(patterns, vec!["x...x...x...x...".to_string()]);
    }

    #[test]
    fn suggest_patterns_still_requires_api_key_for_melodic_tracks() {
        let cfg = AiConfig { api_key: None, model: "gpt-5.2".to_string() };
        let ctx = PatternContext {
            bpm: 120,
            steps: 16,
            target_track: "synth".to_string(),
            tracks: vec![],
        };

        let err = suggest_patterns(&cfg, "4 on the floor", &ctx).unwrap_err();
        assert!(err.to_string().contains("OPENAI_API_KEY"));
    }
}
