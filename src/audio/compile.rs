use crate::pattern::visual::{parse_visual_pattern, Gate, Step, StepEvent};
use super::timing::{ACCENT_VELOCITY, DEFAULT_VELOCITY};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceSpan {
    pub start: usize,
    pub steps: usize,
    pub gate: Option<crate::pattern::visual::Gate>,
}

#[allow(dead_code)]
/// Pre-computes contiguous voice durations to support sustain/tie playback.
pub fn compile_voice_spans(steps: &[Step]) -> Vec<VoiceSpan> {
    let mut out: Vec<VoiceSpan> = Vec::new();
    let mut current: Option<VoiceSpan> = None;
    for (i, step) in steps.iter().enumerate() {
        match step {
            Step::Hit(ev) => {
                if let Some(span) = current.take() {
                    out.push(span);
                }
                current = Some(VoiceSpan { start: i, steps: 1, gate: ev.gate });
            }
            Step::Tie => {
                if let Some(ref mut span) = current {
                    span.steps += 1;
                }
            }
            Step::Chord(events) => {
                if let Some(span) = current.take() { out.push(span); }
                for _ev in events {
                    out.push(VoiceSpan { start: i, steps: 1, gate: None });
                }
            }
            Step::Rest => {
                if let Some(span) = current.take() {
                    out.push(span);
                }
            }
        }
    }
    if let Some(span) = current { out.push(span); }
    out
}

/// A single compiled event (one voice to trigger).
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledEvent {
    pub pitch: i32,
    pub velocity: u8,
    pub gate: Option<Gate>,
    pub ratchet: Option<u32>,     // Number of sub-hits (e.g., 3 = triplet fill)
    pub probability: Option<f32>, // Trigger probability 0.0-1.0 (None = always trigger)
}

/// Compiled data for a single step in the pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledStep {
    pub events: Vec<CompiledEvent>, // Empty for rests/ties
    pub hold_steps: usize,          // How many steps to sustain (for gate mode)
}

#[allow(dead_code)]
impl CompiledStep {
    pub fn rest() -> Self {
        Self { events: Vec::new(), hold_steps: 0 }
    }
    
    pub fn hit(pitch: i32, velocity: u8, gate: Option<Gate>, hold_steps: usize) -> Self {
        Self {
            events: vec![CompiledEvent { pitch, velocity, gate, ratchet: None, probability: None }],
            hold_steps,
        }
    }
    
    pub fn is_hit(&self) -> bool {
        !self.events.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledPattern {
    pub steps: Vec<CompiledStep>,
    // Legacy accessors for backward compatibility
    pub triggers: Vec<bool>,
    pub pitches: Vec<Option<i32>>,
    pub hold_steps: Vec<usize>,
    pub gates: Vec<Option<Gate>>,
    pub velocities: Vec<u8>,
}

#[allow(dead_code)]
impl CompiledPattern {
    pub fn empty() -> Self {
        Self {
            steps: Vec::new(),
            triggers: Vec::new(),
            pitches: Vec::new(),
            hold_steps: Vec::new(),
            gates: Vec::new(),
            velocities: Vec::new(),
        }
    }
    
    pub fn len(&self) -> usize {
        self.steps.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

pub fn visual_to_tokens_and_pitches(s: &str) -> CompiledPattern {
    match parse_visual_pattern(s) {
        Ok(steps) => compile_tokens_pitches_and_gates(&steps),
        Err(err) => {
            crate::console::warn(format!("pattern parse error: {err}"));
            let mut compiled_steps: Vec<CompiledStep> = Vec::new();
            let mut triggers: Vec<bool> = Vec::new();
            let mut pitches: Vec<Option<i32>> = Vec::new();
            let mut hold_steps: Vec<usize> = Vec::new();
            let mut gates: Vec<Option<Gate>> = Vec::new();
            let mut velocities: Vec<u8> = Vec::new();
            let mut last_hit: Option<usize> = None;

            for ch in s.chars().filter(|c| !c.is_whitespace()) {
                match ch {
                    'x' | '1' | '*' => {
                        compiled_steps.push(CompiledStep {
                            events: vec![CompiledEvent { pitch: 0, velocity: DEFAULT_VELOCITY, gate: None, ratchet: None, probability: None }],
                            hold_steps: 1,
                        });
                        triggers.push(true);
                        pitches.push(None);
                        hold_steps.push(1);
                        gates.push(None);
                        velocities.push(DEFAULT_VELOCITY);
                        last_hit = Some(triggers.len() - 1);
                    }
                    'X' => {
                        compiled_steps.push(CompiledStep {
                            events: vec![CompiledEvent { pitch: 0, velocity: ACCENT_VELOCITY, gate: None, ratchet: None, probability: None }],
                            hold_steps: 1,
                        });
                        triggers.push(true);
                        pitches.push(None);
                        hold_steps.push(1);
                        gates.push(None);
                        velocities.push(ACCENT_VELOCITY);
                        last_hit = Some(triggers.len() - 1);
                    }
                    '_' => {
                        compiled_steps.push(CompiledStep::rest());
                        triggers.push(false);
                        pitches.push(None);
                        hold_steps.push(0);
                        gates.push(None);
                        velocities.push(0);
                        if let Some(idx) = last_hit {
                            hold_steps[idx] += 1;
                            compiled_steps[idx].hold_steps += 1;
                        }
                    }
                    '.' => {
                        compiled_steps.push(CompiledStep::rest());
                        triggers.push(false);
                        pitches.push(None);
                        hold_steps.push(0);
                        gates.push(None);
                        velocities.push(0);
                        last_hit = None;
                    }
                    '|' => {
                        // Bar separators behave like rests in fallback mode.
                        compiled_steps.push(CompiledStep::rest());
                        triggers.push(false);
                        pitches.push(None);
                        hold_steps.push(0);
                        gates.push(None);
                        velocities.push(0);
                        last_hit = None;
                    }
                    _ => {
                        compiled_steps.push(CompiledStep::rest());
                        triggers.push(false);
                        pitches.push(None);
                        hold_steps.push(0);
                        gates.push(None);
                        velocities.push(0);
                        last_hit = None;
                    }
                }
            }
            CompiledPattern {
                steps: compiled_steps,
                triggers,
                pitches,
                hold_steps,
                gates,
                velocities,
            }
        }
    }
}

/// Compute velocity for a step event based on accent and explicit velocity.
fn event_velocity(ev: &StepEvent) -> u8 {
    if let Some(v) = ev.note.velocity {
        v
    } else if ev.note.accent {
        ACCENT_VELOCITY
    } else {
        DEFAULT_VELOCITY
    }
}

fn compile_tokens_pitches_and_gates(steps: &[Step]) -> CompiledPattern {
    let mut compiled_steps = Vec::with_capacity(steps.len());
    
    // Legacy vectors for backward compatibility
    let mut tokens = Vec::with_capacity(steps.len());
    let mut pitches = Vec::with_capacity(steps.len());
    let mut hold_steps_vec = Vec::with_capacity(steps.len());
    let mut gates = Vec::with_capacity(steps.len());
    let mut velocities = Vec::with_capacity(steps.len());

    for (i, step) in steps.iter().enumerate() {
        match step {
            Step::Hit(ev) => {
                let hold = 1 + count_following_ties(steps, i);
                let vel = event_velocity(ev);
                compiled_steps.push(CompiledStep {
                    events: vec![CompiledEvent {
                        pitch: ev.note.pitch_offset,
                        velocity: vel,
                        gate: ev.gate,
                        ratchet: ev.ratchet,
                        probability: ev.note.probability,
                    }],
                    hold_steps: hold,
                });
                // Legacy
                tokens.push(true);
                pitches.push(Some(ev.note.pitch_offset));
                hold_steps_vec.push(hold);
                gates.push(ev.gate);
                velocities.push(vel);
            }
            Step::Chord(events) => {
                let hold = 1 + count_following_ties(steps, i);
                let chord_events: Vec<CompiledEvent> = events
                    .iter()
                    .map(|e| CompiledEvent {
                        pitch: e.note.pitch_offset,
                        velocity: event_velocity(e),
                        gate: e.gate,
                        ratchet: e.ratchet,
                        probability: e.note.probability,
                    })
                    .collect();
                compiled_steps.push(CompiledStep {
                    events: chord_events,
                    hold_steps: hold,
                });
                // Legacy - just use first event
                let first = events.first();
                tokens.push(true);
                pitches.push(first.map(|e| e.note.pitch_offset));
                hold_steps_vec.push(hold);
                gates.push(first.and_then(|e| e.gate));
                velocities.push(first.map(|e| event_velocity(e)).unwrap_or(DEFAULT_VELOCITY));
            }
            Step::Rest | Step::Tie => {
                compiled_steps.push(CompiledStep::rest());
                tokens.push(false);
                pitches.push(None);
                hold_steps_vec.push(0);
                gates.push(None);
                velocities.push(0);
            }
        }
    }

    CompiledPattern {
        steps: compiled_steps,
        triggers: tokens,
        pitches,
        hold_steps: hold_steps_vec,
        gates,
        velocities,
    }
}

fn count_following_ties(steps: &[Step], start: usize) -> usize {
    let mut count = 0;
    let mut idx = start + 1;
    while let Some(step) = steps.get(idx) {
        match step {
            Step::Tie => count += 1,
            _ => break,
        }
        idx += 1;
    }
    count
}

#[allow(dead_code)]
/// Convenience helper retained for forthcoming runtime compilation paths.
pub fn compile_tokens_and_pitches(s: &str) -> (Vec<bool>, Vec<Option<i32>>) {
    match parse_visual_pattern(s) {
        Ok(steps) => {
            let compiled = compile_tokens_pitches_and_gates(&steps);
            (compiled.triggers, compiled.pitches)
        }
        Err(_) => (Vec::new(), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_and_pitches_respect_offsets() {
        let compiled = visual_to_tokens_and_pitches("x . x+7 .");
        assert_eq!(compiled.triggers, vec![true, false, true, false]);
        assert_eq!(compiled.pitches, vec![Some(0), None, Some(7), None]);
    }

    #[test]
    fn hold_steps_capture_ties() {
        let compiled = visual_to_tokens_and_pitches("x___.");
        assert_eq!(compiled.hold_steps, vec![4, 0, 0, 0, 0]);
    }

    #[test]
    fn voice_spans_merge_ties() {
        let steps = parse_visual_pattern("x__.").expect("parse");
        let spans = compile_voice_spans(&steps);
        assert_eq!(spans.len(), 1);
        let span = spans[0];
        assert_eq!(span.start, 0);
        assert_eq!(span.steps, 3);
    }

    #[test]
    fn velocities_default_to_100() {
        let compiled = visual_to_tokens_and_pitches("x . x .");
        assert_eq!(compiled.velocities, vec![DEFAULT_VELOCITY, 0, DEFAULT_VELOCITY, 0]);
    }

    #[test]
    fn accent_uses_accent_velocity() {
        let compiled = visual_to_tokens_and_pitches("x X x");
        assert_eq!(compiled.velocities[0], DEFAULT_VELOCITY);
        assert_eq!(compiled.velocities[1], ACCENT_VELOCITY);
        assert_eq!(compiled.velocities[2], DEFAULT_VELOCITY);
    }

    #[test]
    fn explicit_velocity_overrides_default() {
        let compiled = visual_to_tokens_and_pitches("xv64 . xv127");
        assert_eq!(compiled.velocities[0], 64);
        assert_eq!(compiled.velocities[1], 0);
        assert_eq!(compiled.velocities[2], 127);
    }

    #[test]
    fn accent_with_explicit_velocity_uses_explicit() {
        // Explicit velocity takes precedence even on accented note
        let compiled = visual_to_tokens_and_pitches("Xv50");
        assert_eq!(compiled.velocities[0], 50);
    }

    #[test]
    fn chord_compiles_multiple_events() {
        // Chord notation: (x x+4 x+7) creates a chord with root, major third, fifth
        let compiled = visual_to_tokens_and_pitches("(x x+4 x+7)");
        assert_eq!(compiled.steps.len(), 1);
        assert_eq!(compiled.steps[0].events.len(), 3);
        // Events should be sorted by pitch (0, 4, 7)
        assert_eq!(compiled.steps[0].events[0].pitch, 0);
        assert_eq!(compiled.steps[0].events[1].pitch, 4);
        assert_eq!(compiled.steps[0].events[2].pitch, 7);
    }

    #[test]
    fn chord_with_velocity_per_note() {
        let compiled = visual_to_tokens_and_pitches("(xv80 x+4v100 x+7v60)");
        assert_eq!(compiled.steps.len(), 1);
        assert_eq!(compiled.steps[0].events.len(), 3);
        assert_eq!(compiled.steps[0].events[0].velocity, 80);
        assert_eq!(compiled.steps[0].events[1].velocity, 100);
        assert_eq!(compiled.steps[0].events[2].velocity, 60);
    }

    #[test]
    fn ratchet_compiles_to_event() {
        let compiled = visual_to_tokens_and_pitches("x{3} . x{4}");
        assert_eq!(compiled.steps.len(), 3);
        assert_eq!(compiled.steps[0].events[0].ratchet, Some(3));
        assert!(compiled.steps[1].events.is_empty()); // rest
        assert_eq!(compiled.steps[2].events[0].ratchet, Some(4));
    }

    #[test]
    fn no_ratchet_is_none() {
        let compiled = visual_to_tokens_and_pitches("x . x");
        assert_eq!(compiled.steps[0].events[0].ratchet, None);
        assert_eq!(compiled.steps[2].events[0].ratchet, None);
    }

    #[test]
    fn probability_compiles_to_event() {
        let compiled = visual_to_tokens_and_pitches("x?50% . x?0.25");
        assert_eq!(compiled.steps.len(), 3);
        assert!((compiled.steps[0].events[0].probability.unwrap() - 0.5).abs() < 0.01);
        assert!(compiled.steps[1].events.is_empty()); // rest
        assert!((compiled.steps[2].events[0].probability.unwrap() - 0.25).abs() < 0.01);
    }

    #[test]
    fn no_probability_is_none() {
        let compiled = visual_to_tokens_and_pitches("x . x");
        assert_eq!(compiled.steps[0].events[0].probability, None);
    }

    #[test]
    fn parse_error_is_reported_via_console_warn() {
        let sub = crate::console::subscribe();

        // `^` is not valid visual pattern syntax; this must trigger the fallback path.
        let _compiled = visual_to_tokens_and_pitches("x^..............."); // 16-ish chars, but invalid

        let msgs = sub.drain();
        assert!(
            msgs.iter().any(|m| {
                m.level == crate::console::Level::Warn
                    && m.text.contains("pattern parse error")
                    && m.text.contains('^')
            }),
            "expected a warn log containing the parse error; got: {msgs:?}"
        );
    }
}
