//! Rhai-based scripted patterns for generative pattern creation.

use anyhow::{anyhow, Result};
use rhai::{Engine, EvalAltResult, Scope};

/// Pattern scripting engine wrapping Rhai.
pub struct PatternEngine {
    engine: Engine,
}

impl PatternEngine {
    /// Create a new pattern scripting engine with built-in functions.
    pub fn new() -> Self {
        let mut engine = Engine::new();
        
        // Register built-in pattern generation functions
        engine.register_fn("euclid", euclid);
        engine.register_fn("random", random_pattern);
        engine.register_fn("fill", fill_pattern);
        engine.register_fn("repeat", repeat_pattern);
        engine.register_fn("invert", invert_pattern);
        engine.register_fn("rotate", rotate_pattern);
        engine.register_fn("humanize", humanize_pattern);
        
        // Register string manipulation helpers
        engine.register_fn("len", |s: &str| s.len() as i64);
        
        Self { engine }
    }
    
    /// Evaluate a Rhai script and return the resulting pattern string.
    /// The script should return a String that represents a visual pattern.
    pub fn eval(&self, script: &str) -> Result<String> {
        let result: String = self.engine
            .eval(script)
            .map_err(|e| anyhow!("Script error: {}", format_rhai_error(&e)))?;
        Ok(result)
    }
    
    /// Evaluate a script with variables in scope (e.g., bar number, BPM).
    pub fn eval_with_context(&self, script: &str, bar: i64, bpm: i64) -> Result<String> {
        let mut scope = Scope::new();
        scope.push("bar", bar);
        scope.push("bpm", bpm);
        
        let result: String = self.engine
            .eval_with_scope(&mut scope, script)
            .map_err(|e| anyhow!("Script error: {}", format_rhai_error(&e)))?;
        Ok(result)
    }
}

impl Default for PatternEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn format_rhai_error(e: &EvalAltResult) -> String {
    e.to_string()
}

// --- Built-in pattern generators ---

/// Generate a Euclidean rhythm pattern.
/// `k` = number of hits, `n` = total steps.
fn euclid(k: i64, n: i64) -> String {
    if n <= 0 { return String::new(); }
    if k <= 0 { return ".".repeat(n as usize); }
    if k >= n { return "x".repeat(n as usize); }
    
    let k = k as usize;
    let n = n as usize;
    
    // Bresenham-style Euclidean distribution
    let mut pattern = vec![false; n];
    let mut bucket = 0i32;
    
    for i in 0..n {
        bucket += k as i32;
        if bucket >= n as i32 {
            bucket -= n as i32;
            pattern[i] = true;
        }
    }
    
    // Rotate to start on a hit
    let first_hit = pattern.iter().position(|&x| x).unwrap_or(0);
    pattern.rotate_left(first_hit);
    
    pattern.iter().map(|&hit| if hit { 'x' } else { '.' }).collect()
}

/// Generate a random pattern with given density (0.0-1.0) and optional seed.
fn random_pattern(density: f64, seed: i64) -> String {
    let n = 16; // Default to 16 steps
    random_pattern_n(density, seed, n)
}

fn random_pattern_n(density: f64, seed: i64, n: i64) -> String {
    if n <= 0 { return String::new(); }
    let density = density.clamp(0.0, 1.0);
    
    // Simple LCG random
    let mut state = seed as u64;
    let mut pattern = String::with_capacity(n as usize);
    
    for _ in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let roll = (state as f64) / (u64::MAX as f64);
        if roll < density {
            pattern.push('x');
        } else {
            pattern.push('.');
        }
    }
    
    pattern
}

/// Generate a drum fill pattern of given length.
fn fill_pattern(length: i64) -> String {
    if length <= 0 { return String::new(); }
    
    // Common fill: increasing density toward the end
    match length {
        1..=4 => "x".repeat(length as usize),
        5..=8 => format!("{}x.x.x.xx", ".".repeat((length - 7).max(0) as usize)),
        9..=12 => "x.x.x.x.x.x.".chars().take(length as usize).collect(),
        _ => format!("{}{}", "x.x.".repeat((length / 4) as usize), "x.".repeat(((length % 4) / 2) as usize)),
    }
}

/// Repeat a pattern string n times.
fn repeat_pattern(pattern: &str, n: i64) -> String {
    if n <= 0 { return String::new(); }
    pattern.repeat(n as usize)
}

/// Invert a pattern (x -> ., . -> x).
fn invert_pattern(pattern: &str) -> String {
    pattern.chars().map(|c| match c {
        'x' | 'X' => '.',
        '.' => 'x',
        other => other,
    }).collect()
}

/// Rotate a pattern by n steps.
fn rotate_pattern(pattern: &str, n: i64) -> String {
    if pattern.is_empty() { return String::new(); }
    let len = pattern.len();
    let n = ((n % len as i64) + len as i64) as usize % len;
    let (a, b) = pattern.split_at(n);
    format!("{}{}", b, a)
}

/// Add humanization (velocity/nudge) to a pattern.
fn humanize_pattern(pattern: &str, amount: f64) -> String {
    let amount = (amount * 20.0).round() as i32; // Scale to velocity range
    pattern.chars().map(|c| {
        if c == 'x' && amount > 0 {
            let vel = 100 + (amount.min(27)); // Cap at 127
            format!("xv{}", vel)
        } else {
            c.to_string()
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euclid_basic() {
        // Euclidean rhythms distribute hits as evenly as possible
        let e4_16 = euclid(4, 16);
        assert_eq!(e4_16.chars().filter(|&c| c == 'x').count(), 4);
        assert_eq!(e4_16.len(), 16);
        
        let e3_8 = euclid(3, 8);
        assert_eq!(e3_8.chars().filter(|&c| c == 'x').count(), 3);
        assert_eq!(e3_8.len(), 8);
        
        let e5_8 = euclid(5, 8);
        assert_eq!(e5_8.chars().filter(|&c| c == 'x').count(), 5);
        assert_eq!(e5_8.len(), 8);
    }

    #[test]
    fn euclid_edge_cases() {
        assert_eq!(euclid(0, 8), "........");
        assert_eq!(euclid(8, 8), "xxxxxxxx");
        assert_eq!(euclid(4, 0), "");
    }

    #[test]
    fn random_pattern_deterministic() {
        let p1 = random_pattern(0.5, 42);
        let p2 = random_pattern(0.5, 42);
        assert_eq!(p1, p2); // Same seed = same pattern
    }

    #[test]
    fn invert_swaps_hits() {
        assert_eq!(invert_pattern("x.x."), ".x.x");
        assert_eq!(invert_pattern("...."), "xxxx");
    }

    #[test]
    fn rotate_shifts_pattern() {
        assert_eq!(rotate_pattern("x...", 1), "...x");
        assert_eq!(rotate_pattern("x...", -1), ".x..");
    }

    #[test]
    fn engine_evals_euclid() {
        let engine = PatternEngine::new();
        let result = engine.eval("euclid(4, 16)").unwrap();
        assert_eq!(result.len(), 16);
        assert_eq!(result.chars().filter(|&c| c == 'x').count(), 4);
    }

    #[test]
    fn engine_evals_expression() {
        let engine = PatternEngine::new();
        let result = engine.eval(r#"euclid(3, 8) + "..""#).unwrap();
        assert_eq!(result.len(), 10); // 8 + 2
        assert_eq!(result.chars().filter(|&c| c == 'x').count(), 3);
    }

    #[test]
    fn engine_with_context() {
        let engine = PatternEngine::new();
        let script = r#"if bar % 4 == 3 { fill(8) } else { "x...x...x...x..." }"#;
        
        let normal = engine.eval_with_context(script, 0, 120).unwrap();
        assert_eq!(normal, "x...x...x...x...");
        
        let fill = engine.eval_with_context(script, 3, 120).unwrap();
        assert!(!fill.is_empty());
    }
}

