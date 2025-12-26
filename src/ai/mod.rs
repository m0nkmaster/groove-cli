//! AI-powered pattern generation using LLMs.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Configuration for the AI pattern generator.
#[derive(Clone)]
pub struct AiConfig {
    /// API endpoint (e.g., "http://localhost:11434/api/generate" for Ollama)
    pub endpoint: String,
    /// Model name (e.g., "llama3.2", "claude-3-sonnet")
    pub model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            // Default to Ollama running locally
            endpoint: "http://localhost:11434/api/generate".to_string(),
            model: "llama3.2".to_string(),
        }
    }
}

/// Request body for Ollama-compatible API.
#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// Response from Ollama-compatible API.
#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

/// Generate pattern suggestions based on a description.
pub fn suggest_patterns(config: &AiConfig, description: &str, context: &PatternContext) -> Result<Vec<String>> {
    let prompt = build_prompt(description, context);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let request = OllamaRequest {
        model: config.model.clone(),
        prompt,
        stream: false,
    };
    
    let response = client
        .post(&config.endpoint)
        .json(&request)
        .send()
        .map_err(|e| anyhow!("AI request failed: {}. Is Ollama running?", e))?;
    
    if !response.status().is_success() {
        return Err(anyhow!("AI API error: {}", response.status()));
    }
    
    let result: OllamaResponse = response.json()
        .map_err(|e| anyhow!("Failed to parse AI response: {}", e))?;
    
    // Parse patterns from response
    let patterns = extract_patterns(&result.response);
    
    if patterns.is_empty() {
        Err(anyhow!("No valid patterns in AI response"))
    } else {
        Ok(patterns)
    }
}

/// Context for pattern generation (BPM, existing patterns, etc.).
pub struct PatternContext {
    pub bpm: u32,
    pub steps: usize,
    pub genre_hint: Option<String>,
    pub other_patterns: Vec<String>,
}

impl Default for PatternContext {
    fn default() -> Self {
        Self {
            bpm: 120,
            steps: 16,
            genre_hint: None,
            other_patterns: Vec::new(),
        }
    }
}

fn build_prompt(description: &str, context: &PatternContext) -> String {
    let mut prompt = format!(
        r#"You are a drum pattern generator for a music sequencer.
Generate 3 drum/rhythm patterns for: "{}"

Context:
- BPM: {}
- Steps: {} (where x = hit, . = rest)
- Pattern notation examples: "x...x...x...x..." (basic 4/4), "x.x.x.x.x.x.x.x." (8th notes)

"#,
        description, context.bpm, context.steps
    );
    
    if let Some(ref genre) = context.genre_hint {
        prompt.push_str(&format!("- Genre: {}\n", genre));
    }
    
    if !context.other_patterns.is_empty() {
        prompt.push_str("- Other patterns in the song:\n");
        for p in &context.other_patterns {
            prompt.push_str(&format!("  {}\n", p));
        }
    }
    
    prompt.push_str(r#"
Respond with exactly 3 patterns, one per line, using only 'x' and '.' characters.
Each pattern should be exactly "#);
    prompt.push_str(&context.steps.to_string());
    prompt.push_str(r#" characters long.
Example response format:
x...x...x...x...
x.x...x.x...x...
x..x..x..x..x..x
"#);
    
    prompt
}

/// Extract valid patterns from LLM response.
fn extract_patterns(response: &str) -> Vec<String> {
    response
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .filter(|line| line.chars().all(|c| c == 'x' || c == 'X' || c == '.'))
        .filter(|line| line.len() >= 4) // At least 4 steps
        .map(|s| s.to_lowercase())
        .take(5) // Max 5 suggestions
        .collect()
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
................
"#;
        let patterns = extract_patterns(response);
        assert_eq!(patterns.len(), 3);
        assert_eq!(patterns[0], "x...x...x...x...");
        assert_eq!(patterns[1], "x.x.x.x.x.x.x.x.");
        assert_eq!(patterns[2], "................");
    }

    #[test]
    fn build_prompt_includes_context() {
        let ctx = PatternContext {
            bpm: 140,
            steps: 16,
            genre_hint: Some("techno".to_string()),
            other_patterns: vec!["x...x...x...x...".to_string()],
        };
        let prompt = build_prompt("punchy kick", &ctx);
        assert!(prompt.contains("140"));
        assert!(prompt.contains("techno"));
        assert!(prompt.contains("punchy kick"));
    }
}

