use groove_cli::pattern::visual::{parse_visual_pattern, Gate};
use groove_cli::audio::compile::{compile_voice_spans, visual_to_tokens_and_pitches, VoiceSpan};
use groove_cli::storage::song::open as open_song;
use groove_cli::model::{pattern::Pattern, song::Song};
use groove_cli::audio::timing::gate_duration;
use std::time::Duration;

#[test]
fn compiles_sustain_across_ties() {
    let steps = parse_visual_pattern("x__.").expect("parse");
    let spans = compile_voice_spans(&steps);
    assert_eq!(spans.len(), 1);
    let VoiceSpan { start, steps, .. } = spans[0];
    assert_eq!((start, steps), (0, 3));
}

#[test]
fn gate_fraction_sets_duration() {
    // 120 BPM, div=4 => base step 0.125s; 3/4 gate => 0.09375s
    let base = Duration::from_secs_f64(0.125);
    let d = gate_duration(base, Gate::Fraction { numerator: 3, denominator: 4 });
    assert!((d.as_secs_f64() - 0.125 * 0.75).abs() < 1e-9);
}

#[test]
fn visual_pattern_sustain_spans_full_ties() {
    let compiled = visual_to_tokens_and_pitches("x_______........");
    assert_eq!(compiled.hold_steps.get(0).copied(), Some(8));
}

#[test]
fn synth_yaml_pattern_sustain_matches_expected_ties() {
    let song = open_song("tests/fixtures/sustain.yaml").expect("load sustain fixture");
    let pattern = match song.tracks[0].pattern.as_ref() {
        Some(Pattern::Visual(p)) => p,
        _ => panic!("expected visual pattern"),
    };
    let compiled = visual_to_tokens_and_pitches(pattern);
    assert_eq!(compiled.hold_steps.get(0).copied(), Some(8));
}

#[test]
fn yaml_visual_pattern_with_ties_parses_hold_steps() {
    let yaml = r#"
bpm: 120
steps: 16
swing: 0
repeat: true
tracks:
- name: Test
  sample: samples/synth/C4.wav
  delay:
    on: false
    time: 1/4
    feedback: 0.0
    mix: 0.0
  pattern: !Visual x_______........
  mute: false
  solo: false
  playback: gate
  gain_db: 0.0
  div: 4
"#;
    let song: Song = serde_yaml::from_str(yaml).expect("parse yaml");
    let pattern = match song.tracks[0].pattern.as_ref() {
        Some(Pattern::Visual(p)) => p,
        _ => panic!("expected visual pattern"),
    };
    let compiled = visual_to_tokens_and_pitches(pattern);
    assert_eq!(compiled.hold_steps.get(0).copied(), Some(8));
}

#[test]
fn yaml_visual_pattern_multiline_ties_compile() {
    let yaml = r#"
bpm: 120
steps: 16
swing: 0
repeat: true
tracks:
  - name: Test
    sample: samples/synth/C4.wav
    delay:
      on: false
      time: 1/4
      feedback: 0.0
      mix: 0.0
    pattern: !Visual
      x_______........
    mute: false
    solo: false
    playback: gate
    gain_db: 0.0
    div: 4
"#;
    let song: Song = serde_yaml::from_str(yaml).expect("parse yaml");
    let pattern = match song.tracks[0].pattern.as_ref() {
        Some(Pattern::Visual(p)) => p,
        _ => panic!("expected visual pattern"),
    };
    let compiled = visual_to_tokens_and_pitches(pattern);
    assert_eq!(compiled.hold_steps.get(0).copied(), Some(8));
}

#[test]
fn fallback_sustain_handles_ties_on_parse_error() {
    let compiled = visual_to_tokens_and_pitches("x_______@........");
    assert_eq!(compiled.hold_steps.get(0).copied(), Some(8));
}
