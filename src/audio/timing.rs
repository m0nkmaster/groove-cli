use std::time::Duration;
use crate::pattern::visual::Gate;

// --- Swing helpers (pure, testable) ---
pub fn base_step_period(bpm: u32, div: u32) -> Duration {
    Duration::from_secs_f64(60.0 / bpm as f64 / div.max(1) as f64)
}

fn swing_fraction(swing_percent: u8) -> f64 {
    // Map 0..100 → 0.0..0.5 (0% no swing; 100% extreme 50/150 split)
    (swing_percent as f64 / 100.0).min(1.0) * 0.5
}

pub fn step_period_with_swing(bpm: u32, div: u32, swing_percent: u8, token_index: usize) -> Duration {
    let base = base_step_period(bpm, div);
    if swing_percent == 0 || div == 0 {
        return base;
    }
    // Apply swing as alternating long/short steps. Use even/odd token index.
    let f = swing_fraction(swing_percent);
    let base_sec = base.as_secs_f64();
    let factor = if token_index.is_multiple_of(2) { 1.0 + f } else { 1.0 - f };
    Duration::from_secs_f64(base_sec * factor)
}

#[allow(dead_code)]
/// Computes the playable duration for a step once gate modifiers are applied.
pub fn gate_duration(step_period: Duration, gate: Gate) -> Duration {
    match gate {
        Gate::Fraction { numerator, denominator } => {
            if denominator == 0 { return Duration::from_millis(0); }
            let frac = numerator as f64 / denominator as f64;
            Duration::from_secs_f64(step_period.as_secs_f64() * frac)
        }
        Gate::Percent(p) => {
            let frac = (p as f64 / 100.0).clamp(0.0, 1.0);
            Duration::from_secs_f64(step_period.as_secs_f64() * frac)
        }
        Gate::Float(f) => {
            let frac = (f as f64).clamp(0.0, 1.0);
            Duration::from_secs_f64(step_period.as_secs_f64() * frac)
        }
    }
}

pub fn pitch_semitones_to_speed(semi: i32) -> f32 {
    2f32.powf(semi as f32 / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_step_periods_match() {
        let base = base_step_period(120, 4).as_secs_f64();
        assert!((base - 0.125).abs() < 1e-9);
    }

    #[test]
    fn swing_extremes_behave() {
        let base = base_step_period(120, 4).as_secs_f64();
        let long = step_period_with_swing(120, 4, 100, 0).as_secs_f64();
        let short = step_period_with_swing(120, 4, 100, 1).as_secs_f64();
        assert!((long - base * 1.5).abs() < 1e-9);
        assert!((short - base * 0.5).abs() < 1e-9);
        assert!(((long + short) - (base * 2.0)).abs() < 1e-9);
    }

    #[test]
    fn swing_midpoint_balances() {
        let base = base_step_period(100, 4).as_secs_f64();
        let long = step_period_with_swing(100, 4, 50, 2).as_secs_f64();
        let short = step_period_with_swing(100, 4, 50, 3).as_secs_f64();
        assert!((long - base * 1.25).abs() < 1e-9);
        assert!((short - base * 0.75).abs() < 1e-9);
        assert!(((long + short) - (base * 2.0)).abs() < 1e-9);
    }

    #[test]
    fn gate_duration_respects_fraction() {
        let base = base_step_period(120, 4);
        let g = gate_duration(base, Gate::Fraction { numerator: 3, denominator: 4 });
        assert!((g.as_secs_f64() - base.as_secs_f64() * 0.75).abs() < 1e-9);
    }

    #[test]
    fn pitch_speed_mapping_basic() {
        assert!((pitch_semitones_to_speed(0) - 1.0).abs() < 1e-6);
        assert!((pitch_semitones_to_speed(12) - 2.0).abs() < 1e-6);
        assert!((pitch_semitones_to_speed(-12) - 0.5).abs() < 1e-6);
    }
}
