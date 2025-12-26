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
    let factor = if token_index % 2 == 0 { 1.0 + f } else { 1.0 - f };
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

/// Convert MIDI velocity (0-127) to amplitude gain factor.
/// Uses a curve that feels musical: velocity 100 is unity gain (1.0),
/// velocity 127 is slightly boosted, and velocity 0 is silent.
pub fn velocity_to_gain(velocity: u8) -> f32 {
    if velocity == 0 {
        return 0.0;
    }
    // Map 0-127 to gain where 100 = 1.0, 127 ≈ 1.27, 64 ≈ 0.64
    (velocity as f32) / 100.0
}

/// Default velocity for accented notes (X instead of x).
pub const ACCENT_VELOCITY: u8 = 110;

/// Default velocity for normal hits when no velocity is specified.
pub const DEFAULT_VELOCITY: u8 = 100;

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

    #[test]
    fn velocity_to_gain_basic() {
        // Zero velocity is silent
        assert_eq!(velocity_to_gain(0), 0.0);
        // Default velocity (100) is unity gain
        assert!((velocity_to_gain(100) - 1.0).abs() < 1e-6);
        // Max velocity (127) is boosted
        assert!((velocity_to_gain(127) - 1.27).abs() < 1e-6);
        // Mid velocity
        assert!((velocity_to_gain(64) - 0.64).abs() < 1e-6);
        // Monotonic: higher velocity = higher gain
        assert!(velocity_to_gain(80) < velocity_to_gain(100));
        assert!(velocity_to_gain(100) < velocity_to_gain(127));
    }

    #[test]
    fn accent_velocity_is_louder_than_default() {
        assert!(ACCENT_VELOCITY > DEFAULT_VELOCITY);
    }
}
