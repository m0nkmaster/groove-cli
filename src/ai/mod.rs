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
    #[allow(dead_code)]
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
        r#"You are a drum pattern generator. Generate exactly 1 pattern for: "{}"

SYNTAX: x=hit, X=accent, .=rest (exactly {} characters, no spaces)

EXAMPLES:
x...x...x...x... (4-on-floor)
x.x.x.x.x.x.x.x. (8th notes)
X...x...X...x... (accented)

BPM: {}
"#,
        description, context.steps, context.bpm
    );
    
    if !context.other_patterns.is_empty() {
        prompt.push_str("Complement: ");
        prompt.push_str(&context.other_patterns.first().unwrap_or(&String::new()));
        prompt.push('\n');
    }
    
    prompt.push_str(&format!(
        "\nReply with ONLY the {}-character pattern. No explanation.",
        context.steps
    ));
    
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
            genre_hint: None,
            other_patterns: vec!["x...x...x...x...".to_string()],
        };
        let prompt = build_prompt("punchy kick", &ctx);
        assert!(prompt.contains("140"));
        assert!(prompt.contains("punchy kick"));
        assert!(prompt.contains("x...x...x...x..."));
    }
}

