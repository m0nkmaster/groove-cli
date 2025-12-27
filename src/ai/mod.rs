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

/// OpenAI chat completions request
#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

/// OpenAI chat completions response
#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

/// Generate pattern suggestions based on a description.
pub fn suggest_patterns(config: &AiConfig, description: &str, context: &PatternContext) -> Result<Vec<String>> {
    // Try keyword-based patterns first for reliability
    if let Some(pattern) = keyword_pattern(description, context.steps) {
        return Ok(vec![pattern]);
    }
    
    let api_key = config.api_key.as_ref()
        .ok_or_else(|| anyhow!("OPENAI_API_KEY not set in src/.env"))?;
    
    let prompt = build_prompt(description, context);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let request = OpenAIRequest {
        model: config.model.clone(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: "You generate drum patterns. Reply with ONLY the pattern using x (hit) and . (rest). No explanation.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        max_tokens: 50,
    };
    
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
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
    
    let result: OpenAIResponse = response.json()
        .map_err(|e| anyhow!("Failed to parse OpenAI response: {}", e))?;
    
    let content = result.choices.first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();
    
    // Parse patterns from response
    let patterns = extract_patterns(&content);
    
    if patterns.is_empty() {
        Err(anyhow!("No valid patterns in AI response: {}", content))
    } else {
        Ok(patterns)
    }
}

/// Match common descriptions to reliable patterns.
fn keyword_pattern(desc: &str, steps: usize) -> Option<String> {
    let d = desc.to_lowercase();
    
    let pattern = if d.contains("4") && (d.contains("floor") || d.contains("beat") || d.contains("kick")) {
        "x...x...x...x..."
    } else if d.contains("simple") && d.contains("kick") {
        "x...x...x...x..."
    } else if d.contains("offbeat") {
        "..x...x...x...x."
    } else if d.contains("8th") || d.contains("eighth") {
        "x.x.x.x.x.x.x.x."
    } else if d.contains("16th") || d.contains("sixteenth") {
        "xxxxxxxxxxxxxxxx"
    } else if d.contains("sparse") {
        "x.......x......."
    } else if d.contains("syncopat") {
        "x..x..x...x..x.."
    } else if d.contains("backbeat") || (d.contains("snare") && d.contains("2") && d.contains("4")) {
        "....x.......x..."
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

/// Track info for AI context
pub struct TrackInfo {
    pub name: String,
    pub sample: Option<String>,
    pub pattern: Option<String>,
    pub muted: bool,
    pub gain_db: f32,
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

fn build_prompt(description: &str, context: &PatternContext) -> String {
    let mut prompt = format!(
        r#"Generate a {}-step drum pattern for track "{}" described as: "{}"

SONG CONTEXT:
- BPM: {}
- Steps per bar: {}
"#,
        context.steps, context.target_track, description, context.bpm, context.steps
    );
    
    // Add existing tracks
    if !context.tracks.is_empty() {
        prompt.push_str("\nEXISTING TRACKS:\n");
        for t in &context.tracks {
            let sample_name = t.sample.as_ref()
                .and_then(|s| s.split('/').last())
                .unwrap_or("no sample");
            let pattern = t.pattern.as_deref().unwrap_or("no pattern");
            let status = if t.muted { " (muted)" } else { "" };
            prompt.push_str(&format!("- {}: {} | {}{}\n", t.name, sample_name, pattern, status));
        }
    }
    
    prompt.push_str(&format!(r#"
RULES:
- Use x (hit) and . (rest) only
- Exactly {} characters
- Complement the existing tracks rhythmically
- Match the description's feel

Reply with ONLY the {}-character pattern:
"#, context.steps, context.steps));
    
    prompt
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
    s.chars().all(|c| matches!(c, 'x' | 'X' | '.' | '_' | '|'))
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
    fn build_prompt_includes_context() {
        let ctx = PatternContext {
            bpm: 140,
            steps: 16,
            target_track: "kick".to_string(),
            tracks: vec![
                TrackInfo {
                    name: "drums".to_string(),
                    sample: Some("samples/909/kick.wav".to_string()),
                    pattern: Some("x...x...x...x...".to_string()),
                    muted: false,
                    gain_db: 0.0,
                },
            ],
        };
        let prompt = build_prompt("punchy kick", &ctx);
        assert!(prompt.contains("140"));
        assert!(prompt.contains("16"));
        assert!(prompt.contains("punchy kick"));
        assert!(prompt.contains("kick"));
        assert!(prompt.contains("drums"));
        assert!(prompt.contains("x...x...x...x..."));
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

