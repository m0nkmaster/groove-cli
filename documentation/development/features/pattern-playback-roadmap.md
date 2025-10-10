# Pattern Playback Roadmap

This document outlines the prioritized implementation plan to fully support the visual pattern notation in playback. It focuses on what matters most for making good songs, with an incremental, test-driven path. Parsing is already implemented; this roadmap covers how to realize those semantics in the sequencer.

Order of implementation (highest musical impact first):
1) Pitch + Sustain (gate/tie/legato)
2) Velocity + Accent
3) Chords / Polyphony
4) Ratchets
5) Probability
6) Nudge (timing offsets)
7) Cycle Conditions
8) Param Locks (per-step FX)

Notes:
- Today playback reduces patterns to a boolean grid (hit/rest). All modifiers are ignored at runtime.
- We’ll replace that with an event-aware scheduler that honors the parsed `Step`/`StepEvent` data.
- All features below include acceptance criteria and TDD guidance.

## 0. Foundations (refactor target)

Goal: Introduce an internal representation that supports timing, dynamics, and per-step options without exploding complexity.

- Data model additions
  - Replace `Vec<bool>` patterns with `Vec<Step>` or a compiled form.
  - Introduce `CompiledStep` for runtime scheduling:
    - `start_tick: usize` (grid position), `hits: SmallVec<Hit>`
    - `Hit { offsets: Vec<i32>, velocity: Option<u8>, accent: bool, gate: Option<Gate>, nudge: Option<Nudge>, ratchet: Option<u32>, cycle: Option<CycleCondition>, param_locks: Vec<ParamLock> }`
  - Keep compilation simple: one `CompiledStep` per visual step index. Ratchets expand at runtime.

- Scheduling
  - Drive timing from current BPM/div, with per-token swing.
  - On each tick boundary, evaluate the step’s hits for triggering, cycle tests, probability, and build one or more sample `voices` with timing adjustments.

- Testing
  - Unit-test `compile_visual_to_steps` and pure helpers.
  - Integration-test scheduling with a fake clock and a mocked sink that records trigger timestamps and durations.

Acceptance:
- No behavior change for plain `x`/`.` patterns.
- Existing tests remain green.

## 1. Pitch + Sustain (Gate/Tie/Legato)

Why first: Defines musical contour and note length—core to phrasing.

- Pitch
  - Support `+N`/`-N` offsets for samples.
  - Implementation strategies:
    - Basic (MVP): treat pitch offset as metadata (no pitch shift) unless a sample map exists.
    - Optional: add resampling-based pitch shift using `Source::speed` (not in rodio stable) or an external resampler. Alternatively, map offsets to multi-samples (preferred for drums). Start with metadata-only; expose hooks for future pitch engines.

- Sustain/Gate
  - Implement `Gate::Fraction/Percent/Float` by truncating playback via `Source::take_duration` per voice to `gate * step_duration`.
  - Handle `_` tie: if previous step is a hit and current is `_`, extend the prior voice instead of retriggering.
  - Legato rule: hit followed by tie(s) sustains until first non-tie or pattern end.

- TDD
  - Unit: gate length computation equals fraction of step period under different swings.
  - Unit: tie chain extends duration and suppresses retriggers.
  - Integration: fake sink records one long voice across ties; asserts start count and total duration.

Acceptance:
- Patterns with `=1/2` make audible half-length notes.
- `x__.` plays one trigger sustained across two ties with a single voice instance.

## 2. Velocity + Accent

Why: Dynamics are the second most expressive control after note length.

- Velocity
  - Apply velocity scaling as amplitude gain: map `0..127` to linear gain via `db` or direct factor.
  - Accent `X`: treat as a fixed velocity boost (e.g., clamp at 110–127) when velocity is not specified.

- TDD
  - Unit: `velocity_to_gain(0) == 0`, monotonic mapping.
  - Integration: fake sink captures gain per voice; assert accents louder than normal hits.

Acceptance:
- `x v40 x v100` yields clearly different loudness; `X` louder than `x`.

## 3. Chords / Polyphony

Why: Harmony and stacked hits are essential for musicality.

- Behavior
  - `Chord(Vec<StepEvent>)` triggers multiple voices at the same tick.
  - Each sub-event inherits step gate/tie behavior independently.

- TDD
  - Integration: chord step emits N voices simultaneously; durations honored per event gate.

Acceptance:
- `(x x+4 x+7)` produces 3 concurrent voices; rests and ties around the group don’t add extra retriggers.

## 4. Ratchets

Why: Rhythmic interest; essential for rolls and fills.

- Behavior
  - `{N}` splits the step interval into `N` equal sub-steps and triggers `N` short voices.
  - Gate within ratchets: default gate per sub-step equals parent gate unless overridden; if no gate, use a musical default (e.g., 80% of sub-step length).

- TDD
  - Unit: ratchet scheduler creates `N` triggers within one step duration.
  - Integration: fake sink collects `N` timestamps, equidistant (swing may apply only at step boundary, not within ratchets for v1).

Acceptance:
- `x{3}` fires 3 evenly spaced sub-hits in one step.

## 5. Probability

Why: Make patterns feel alive; good for generative variations.

- Behavior
  - `?p` where `p` is `0..1` or `%` form. On each evaluation, trigger with probability `p`.
  - Determinism: require a seeded RNG per track/session for testability; seed via song or fixed test seed.

- TDD
  - Unit: given a fixed seed, a known pattern of true/false emerges; e.g., the first 10 evaluations match expected.
  - Integration: run a short simulation and assert count of firings equals expected for seed + probability.

Acceptance:
- `x?50%` sometimes fires; with a fixed seed it is reproducible in tests.

## 6. Nudge (Timing Offsets)

Why: Human feel; micro-timing.

- Behavior
  - `@±Nms` shifts event start time by milliseconds relative to step boundary.
  - `@±N%` shifts as a fraction of step duration.
  - For ratchets, apply nudge to each sub-hit relative to its sub-boundary.

- TDD
  - Unit: computed offset equals requested ms/% with swing.
  - Integration: fake sink shows timestamp deviations matching nudges.

Acceptance:
- `x@-5ms` clearly advances relative to neighbors; `%` scales with tempo.

## 7. Cycle Conditions

Why: Musical variations on longer phrases.

- Behavior
  - `@h/d` (parsed as cycle condition) fires on the `h`th hit of every `d` cycles of the same step index.
  - Define cycle counter per track and step index.

- TDD
  - Unit: sequence over 8 bars shows firing pattern 1/4.
  - Integration: with repeat on, counters wrap modulo denominator.

Acceptance:
- `x@1/4` plays only once every 4 cycles at that step.

## 8. Param Locks (Per-Step FX)

Why: Timbre and space automation; high musical impact once core timing is solid.

- Behavior
  - `[key=value, key2]` attaches param changes for the duration of the step/voice.
  - Initial scope: limit to existing `Song` FX (e.g., delay.on, delay.time, delay.mix). Apply at trigger time.
  - Two modes:
    - Per-hit static: set parameter before enqueueing the voice; restore after gate ends.
    - Global/last-write-wins for conflicting events in the same tick.

- TDD
  - Unit: param parsing already tested; add application tests using a mock FX engine that records changes.
  - Integration: when a step with `[delay.on]` fires, mock shows param toggled during voice lifetime.

Acceptance:
- `x[delay.on]` audibly engages delay for that hit; `[delay.time=1/8]` changes time for that hit.

---

## Implementation Details & APIs

- Timing helpers
  - `base_step_period(bpm, div) -> Duration`
  - `step_period_with_swing(bpm, div, swing, token_index) -> Duration` (exists)
  - `gate_duration(step_period, gate) -> Duration`
  - `apply_nudge(instant, nudge, step_period) -> Instant`

- Voice management
  - Wrap decoder with `.amplify(gain)` and `.take_duration(dur)`.
  - Track active voices per track for tie extension (replace the voice’s duration when extended by `_`).
  - For chords, spawn multiple voices; for ratchets, enqueue sub-voices with offsets.

- State
  - Track-level deterministic RNG (seeded in `SequencerConfig`).
  - Cycle counters per `(track_id, step_index)`.

## Testing Strategy (TDD)

- Prefer pure units for all math: gate, swing, nudge, ratchets spacing, probability sequences.
- Provide a `FakeSink` implementing a minimal subset to record `start_time`, `duration`, `gain`, `track`.
- Avoid real audio output in tests; no IO or sleep—advance a fake clock.
- Add golden tests for short patterns demonstrating combined features.

## Incremental Plan

1) Foundations refactor (introduce event-aware pattern in runtime; keep boolean mapping as fallback behind feature flag).
2) Pitch + Sustain
3) Velocity + Accent
4) Chords / Polyphony
5) Ratchets
6) Probability (seeded)
7) Nudge
8) Cycle Conditions
9) Param Locks

Each step lands with passing unit + integration tests and no regressions to existing behavior for simple patterns.

## Risks & Mitigations

- Gate/truncate artifacts: use windowed fade-out to avoid clicks (optional small linear ramp at end of gated voice).
- Pitch shifting quality: start with mapping or metadata; add high-quality resampler later if needed.
- Scheduling drift under heavy CPU: keep loop sleeps short and compute next wake precisely; integration tests cover correctness under simulated time.

## Developer Notes

- Keep features orthogonal in code paths—compose in the scheduler: probability => cycle => ratchet => nudge => gate => param locks.
- Maintain deterministic tests by injecting RNG and clock.
- Document user-facing limits in `documentation/user-guide/pattern-notation.md` as features graduate from planned → implemented.

