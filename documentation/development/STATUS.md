# Development Status

This note captures the current implementation state of `groove-cli` after reviewing the repository documentation and source code. It pairs the shipped functionality with the work that still remains so the team has a clear picture of progress.

## Implemented

- **REPL workflow** – `src/repl/mod.rs` provides the interactive loop with command history, meta commands (`:help`, `:live`, `:q`), chained command parsing, and handlers for tempo, steps, swing, track management, samples, patterns, gain, mute/solo, playback modes, delay parameters, transport, and persistence.
- **Live playing view** – the REPL ticker renders a status header plus per-track grid when `:live on` is set, keeping the cursor stable via the external printer bridge. `clear` resets the region.
- **Visual pattern parsing** – `src/pattern/visual` parses the full visual DSL (pitch offsets, ties, gates, ratchets, velocity markers, probability, cycle conditions, nudges, param locks, chords, repeats, comments). Tests in `tests/visual_pattern.rs` exercise these cases.
- **Sequencer core** – `src/audio.rs` schedules playback on a background thread with swing-aware timing, mono/gate/one-shot playback modes, per-track division, gain control, triggers compiled from patterns, and pitch shifting through `rodio::Source::speed`. Ties expand into sustain durations and optional gate fractions via `audio::compile`.
- **Song persistence & hot reload** – YAML load/save lives in `src/storage/song.rs`; `main.rs` wires up both notify-based watching and a polling fallback so changes to `song.yaml` reload automatically while printing reload notices.
- **Testing** – Unit and integration tests cover visual pattern parsing, sustain compilation, swing timing helpers, playback modes, YAML round-trip, and delay value parsing, giving a baseline safety net (`cargo test`).

## Outstanding / Planned

- **Delay effect rendering** – Delay parameters are configured in the model but never applied in the audio engine. `build_config` ignores `track.delay`; scheduler/playback need to route voices through a delay line per `DEVELOPMENT.md` “Future Work”.
- **Pattern semantics beyond pitch/ties/gate** – The parser recognises velocity, accents, ratchets, probability, nudges, cycle conditions, param locks, and full chords, yet `audio::compile::visual_to_tokens_and_pitches` collapses most of these. The runtime fires a single pitch per step, ignores velocity/accent gain mapping, skips ratchet subdivision, probability, nudges, cycle gating, and param lock execution. Implement the roadmap in `features/pattern-playback-roadmap.md`.
- **Chord polyphony** – Chord steps currently keep only the first event when compiling to runtime tokens, so simultaneous hits are lost. Multi-voice triggering per chord (and param lock scoping) remains undone.
- **Song repeats & steps** – The engine honours `Song.repeat` once set, but there is no REPL command to toggle it and no scheduling based on `Song.steps`; documentation calls out repeat/meter/quantise commands as planned additions.
- **Sample autocomplete & richer REPL UX** – There is no `rustyline` completer for sample paths, directory caching, or method/file completions described in `features/sample-autocomplete.md` and the broader REPL vision (`features/full-spec.md`).
- **Expanded UI/TUI work** – The live view lacks the richer header (`bpm/steps/swing`) and meter panels illustrated in `features/cli-ui-gallery.md`. A full Ratatui-based interface is still future-facing.
- **Additional feature ideas** – The backlog in `features/to-implement.md` (long sample support, chaining helpers, performance commands, FX catalog, envelopes/LFOs, logic, “AI”, etc.) remains untouched in code.
- **Documentation sync** – `documentation/development/features/repl-commands.md` still marks swing as “audio pending”, though `audio::timing::step_period_with_swing` drives timing today. Update docs once further behavior gaps close.

Use this document as the jumping-off point when prioritising the next milestone: finish the pattern playback roadmap, wire delay into the audio engine, surface repeat/steps controls, and close the documentation gaps as functionality lands.
