use groove_cli::pattern::visual::{
    parse_visual_pattern, CycleCondition, Gate, Nudge, ParamLock, Step, StepEvent, StepNote,
};

fn default_note() -> StepNote {
    StepNote {
        pitch_offset: 0,
        velocity: None,
        accent: false,
        probability: None,
        cycle: None,
        param_locks: Vec::new(),
    }
}

fn hit_step(note: StepNote) -> Step {
    Step::Hit(StepEvent {
        note,
        ratchet: None,
        nudge: None,
        gate: None,
    })
}

#[test]
fn parses_hits_rests_ties_and_bars() {
    let pattern = "x...|x.._";
    let steps = parse_visual_pattern(pattern).expect("parse");
    assert_eq!(steps.len(), 7);
    assert_eq!(steps[0], hit_step(default_note()));
    assert!(matches!(steps[1], Step::Rest));
    assert!(matches!(steps[2], Step::Rest));
    assert!(matches!(steps[3], Step::Rest));
    assert_eq!(steps[4], hit_step(default_note()));
    assert!(matches!(steps[5], Step::Rest));
    assert!(matches!(steps[6], Step::Tie));
}

#[test]
fn parses_step_modifiers() {
    let pattern = "x+7?50%v96{3}@-5ms=3/4[delay.time=1/8, delay.on]";
    let steps = parse_visual_pattern(pattern).expect("parse");
    assert_eq!(steps.len(), 1);
    let Step::Hit(event) = &steps[0] else {
        panic!("expected hit");
    };
    assert_eq!(event.note.pitch_offset, 7);
    assert_eq!(event.note.velocity, Some(96));
    assert!(event.note.accent == false);
    let prob = event.note.probability.expect("probability");
    assert!((prob - 0.5).abs() < 1e-6);
    assert_eq!(event.ratchet, Some(3));
    let Nudge::Millis(ms) = event.nudge.expect("nudge") else {
        panic!("nudge unit");
    };
    assert!((ms + 5.0).abs() < 1e-6);
    let Gate::Fraction {
        numerator,
        denominator,
    } = event.gate.expect("gate")
    else {
        panic!("gate form");
    };
    assert_eq!((numerator, denominator), (3, 4));
    assert_eq!(
        event.note.param_locks,
        vec![
            ParamLock {
                key: "delay.time".into(),
                value: Some("1/8".into())
            },
            ParamLock {
                key: "delay.on".into(),
                value: None
            },
        ],
    );
}

#[test]
fn parses_cycle_condition_and_ratchet() {
    let pattern = "x@1/4{4}";
    let steps = parse_visual_pattern(pattern).expect("parse");
    let Step::Hit(event) = &steps[0] else {
        panic!("expected hit");
    };
    let cycle = event.note.cycle.clone().expect("cycle");
    assert_eq!(cycle, CycleCondition { hit: 1, of: 4 });
    assert_eq!(event.ratchet, Some(4));
}

#[test]
fn parses_chord_offsets() {
    let pattern = "x+(0,4,7)";
    let steps = parse_visual_pattern(pattern).expect("parse");
    assert_eq!(steps.len(), 1);
    let Step::Chord(chord) = &steps[0] else {
        panic!("expected chord");
    };
    let offsets: Vec<i32> = chord.iter().map(|ev| ev.note.pitch_offset).collect();
    assert_eq!(offsets, vec![0, 4, 7]);
}

#[test]
fn parses_inline_chord_group() {
    let pattern = "(x x+4 x+7)";
    let steps = parse_visual_pattern(pattern).expect("parse");
    assert_eq!(steps.len(), 1);
    let Step::Chord(chord) = &steps[0] else {
        panic!("expected chord");
    };
    let offsets: Vec<i32> = chord.iter().map(|ev| ev.note.pitch_offset).collect();
    assert_eq!(offsets, vec![0, 4, 7]);
}

#[test]
fn parses_group_repeat() {
    let pattern = "(x.)*2";
    let steps = parse_visual_pattern(pattern).expect("parse");
    assert_eq!(steps.len(), 4);
    assert!(matches!(steps[0], Step::Hit(_)));
    assert!(matches!(steps[1], Step::Rest));
    assert!(matches!(steps[2], Step::Hit(_)));
    assert!(matches!(steps[3], Step::Rest));
}

#[test]
fn ignores_comments() {
    let pattern = "x. # comment\n .x";
    let steps = parse_visual_pattern(pattern).expect("parse");
    assert_eq!(steps.len(), 4);
    assert!(matches!(steps[0], Step::Hit(_)));
    assert!(matches!(steps[1], Step::Rest));
    assert!(matches!(steps[2], Step::Rest));
    assert!(matches!(steps[3], Step::Hit(_)));
}
