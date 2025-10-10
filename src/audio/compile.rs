use crate::pattern::visual::{parse_visual_pattern, Gate, Step};

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

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledPattern {
    pub triggers: Vec<bool>,
    pub pitches: Vec<Option<i32>>,
    pub hold_steps: Vec<usize>,
    pub gates: Vec<Option<Gate>>,
}

impl CompiledPattern {
    pub fn empty() -> Self {
        Self {
            triggers: Vec::new(),
            pitches: Vec::new(),
            hold_steps: Vec::new(),
            gates: Vec::new(),
        }
    }
}

pub fn visual_to_tokens_and_pitches(s: &str) -> CompiledPattern {
    match parse_visual_pattern(s) {
        Ok(steps) => compile_tokens_pitches_and_gates(&steps),
        Err(err) => {
            eprintln!("pattern parse error: {err}");
            let mut triggers: Vec<bool> = Vec::new();
            let mut pitches: Vec<Option<i32>> = Vec::new();
            let mut hold_steps: Vec<usize> = Vec::new();
            let mut gates: Vec<Option<Gate>> = Vec::new();
            let mut last_hit: Option<usize> = None;

            for ch in s.chars().filter(|c| !c.is_whitespace()) {
                match ch {
                    'x' | 'X' | '1' | '*' => {
                        triggers.push(true);
                        pitches.push(None);
                        hold_steps.push(1);
                        gates.push(None);
                        last_hit = Some(triggers.len() - 1);
                    }
                    '_' => {
                        triggers.push(false);
                        pitches.push(None);
                        hold_steps.push(0);
                        gates.push(None);
                        if let Some(idx) = last_hit { hold_steps[idx] += 1; }
                    }
                    '.' => {
                        triggers.push(false);
                        pitches.push(None);
                        hold_steps.push(0);
                        gates.push(None);
                        last_hit = None;
                    }
                    '|' => {
                        // Bar separators behave like rests in fallback mode.
                        triggers.push(false);
                        pitches.push(None);
                        hold_steps.push(0);
                        gates.push(None);
                        last_hit = None;
                    }
                    _ => {
                        triggers.push(false);
                        pitches.push(None);
                        hold_steps.push(0);
                        gates.push(None);
                        last_hit = None;
                    }
                }
            }
            CompiledPattern { triggers, pitches, hold_steps, gates }
        }
    }
}

fn compile_tokens_pitches_and_gates(steps: &[Step]) -> CompiledPattern {
    let mut tokens = Vec::with_capacity(steps.len());
    let mut pitches = Vec::with_capacity(steps.len());
    let mut hold_steps = Vec::with_capacity(steps.len());
    let mut gates = Vec::with_capacity(steps.len());

    for (i, step) in steps.iter().enumerate() {
        match step {
            Step::Hit(ev) => {
                tokens.push(true);
                pitches.push(Some(ev.note.pitch_offset));
                hold_steps.push(1 + count_following_ties(steps, i));
                gates.push(ev.gate);
            }
            Step::Chord(events) => {
                tokens.push(true);
                let off = events.first().map(|e| e.note.pitch_offset).unwrap_or(0);
                pitches.push(Some(off));
                hold_steps.push(1 + count_following_ties(steps, i));
                gates.push(events.first().and_then(|e| e.gate));
            }
            Step::Rest | Step::Tie => {
                tokens.push(false);
                pitches.push(None);
                hold_steps.push(0);
                gates.push(None);
            }
        }
    }

    CompiledPattern { triggers: tokens, pitches, hold_steps, gates }
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
}
