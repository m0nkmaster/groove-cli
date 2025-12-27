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
            model: "gpt-4o-mini".to_string(),
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

/// Generate pattern suggestions based on a description.
pub fn suggest_patterns(config: &AiConfig, description: &str, context: &PatternContext) -> Result<Vec<String>> {
    // Try keyword-based patterns first for reliability
    if let Some(pattern) = keyword_pattern(description, context.steps) {
        return Ok(vec![pattern]);
    }
    
    let api_key = config.api_key.as_ref()
        .ok_or_else(|| anyhow!("OPENAI_API_KEY not set in .env"))?;
    
    let instructions = build_instructions(context);
    let input = build_input(description, context);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let request = ResponsesRequest {
        model: config.model.clone(),
        input,
        instructions,
        max_output_tokens: 100,
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
    
    // Extract text from response
    let content = result.output.first()
        .and_then(|o| o.content.as_ref())
        .and_then(|c| c.first())
        .and_then(|c| c.text.clone())
        .unwrap_or_default();
    
    // Parse patterns from response
    let patterns = extract_patterns(&content);
    
    if patterns.is_empty() {
        Err(anyhow!("No valid patterns in AI response: {}", content))
    } else {
        Ok(patterns)
    }
}

/// Build the system instructions with full context about what we're doing.
fn build_instructions(context: &PatternContext) -> String {
    format!(r#"You are a pattern generator for groove-cli, a command-line step sequencer for creating music.

## WHAT YOU'RE CREATING
You generate step sequences for tracks in a song. Each track plays a sample (audio file) according to its pattern. The patterns loop continuously to create rhythmic music.

## PATTERN SYNTAX
Patterns are strings where each character represents one step (typically a 16th note):
- x or X = trigger/hit (play the sample)
- . = rest/silence (no sound)
- _ = tie/sustain (extend previous note)
- | = bar divider (visual only, ignored by sequencer)

Advanced syntax (optional):
- X = accented hit (louder)
- Grouping: (xx..) subdivides into the time of one step
- Modifiers after x: ^ accent, ~ ghost/soft

## CURRENT SONG
- BPM: {}
- Steps per pattern: {}

## INSTRUMENT HINTS
The track name often indicates the instrument type:
- kick, bass, bd = low frequency, rhythmic foundation
- snare, sd, clap = backbeat, typically beats 2 and 4
- hat, hh, hihat = high frequency, often busy patterns
- perc, conga, shaker = auxiliary rhythm
- lead, synth, melody = melodic content
- pad, strings = sustained, sparse patterns

## MUSICAL GUIDELINES
- Patterns should be musically coherent and complement existing tracks
- Consider the BPM: faster tempos often need simpler patterns
- Leave space - not every step needs a hit
- Syncopation creates groove
- The pattern must be EXACTLY {} characters long

Reply with ONLY the pattern string. No explanation, no quotes, no markdown."#,
        context.bpm, context.steps, context.steps
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
    input.push_str(&format!(
        "Create a {}-step pattern for track \"{}\".\nDescription: {}\n\n\
        Consider how this pattern will work with the existing tracks above.",
        context.steps, context.target_track, description
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
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .filter(|line| is_valid_pattern(line))
        .map(|s| s.to_string())
        .take(3)
        .collect()
}

/// Check if a line looks like a valid pattern.
fn is_valid_pattern(s: &str) -> bool {
    // Must be reasonable length (4-64 chars, no spaces)
    if s.len() < 4 || s.len() > 64 || s.contains(' ') {
        return false;
    }
    // Must contain at least one hit (x/X)
    let has_hits = s.chars().any(|c| matches!(c, 'x' | 'X'));
    if !has_hits {
        return false;
    }
    // All chars must be valid pattern syntax (no spaces)
    s.chars().all(|c| matches!(c, 'x' | 'X' | '.' | '_' | '|' | '^' | '~' | '(' | ')'))
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
}
