# Development Guide

This document is for engineers working on `groove-cli`. It describes the current architecture, module boundaries, and the day-to-day workflow.

## Run

Default (TUI):

```bash
cargo run --
```

Classic REPL mode:

```bash
cargo run -- --repl
```

Open a YAML song on start:

```bash
cargo run -- --open songs/song.yaml
```

Notes:
- In **REPL mode**, when started with `--open …` (or when `song.yaml` / `song.yml` exists in the current directory), the app watches that file and **reloads audio** on changes.
- In **TUI mode**, file watching is not currently enabled.

## Test

```bash
cargo test
```

## High-level architecture

- **UI thread**
  - Default: `src/tui/` (Ratatui + Crossterm event loop).
  - Optional: `src/repl/` classic REPL (`rustyline`) with prompt + history.
  - Both frontends execute the same command handler: `repl::handle_line_for_tui`.
- **Audio thread**
  - `audio::play_song` spawns a background transport thread and stores a `Sender` in a global `OnceCell`/`Mutex` so the UI can send `Stop`/`Update` messages (`audio::reload_song`).

## Key modules

- `src/main.rs`
  - CLI flags (`--open`, `--quiet`, `--repl`)
  - Selects **TUI by default**, REPL when `--repl` is set
  - REPL-only file watching + polling fallback
- `src/tui/`
  - Tracker-style UI, input line, message log
  - Uses tab completion via `repl::completer::complete_for_tui`
- `src/repl/`
  - Command parsing + execution (`handle_line`)
  - Track-first commands (e.g. `kick x...`, `kick ~ …`) and index-based commands (e.g. `pattern 1 …`)
  - Dot-chaining parser: `track("Kick").sample(1, "...").pattern(1, "...")`
  - `:live` ticker for the classic REPL (prints an ANSI grid driven by `audio::snapshot_live_state`)
- `src/pattern/visual/`
  - Visual pattern parser (`parse_visual_pattern`)
  - Supports hits/rests/ties, pitch, velocity, probability, ratchets, gates, chords, groups/repeats, comments
- `src/audio/`
  - `compile.rs`: compiles visual patterns into a runtime-friendly `CompiledPattern`
  - `effects.rs`: delay implementation and delay-time parsing
  - `timing.rs`: swing timing, pitch-to-speed, velocity-to-gain, gate math
- `src/storage/`
  - YAML save/load (`serde_yaml`) via `storage::song::{save, open}`
- `src/ai/`
  - AI pattern generation via the OpenAI Responses API (requires `OPENAI_API_KEY`)
- `src/model/`
  - `Song`, `Track`, `Pattern`, and FX model types used by both UI and audio

## Audio behavior (current)

- **Per-track division**: `Track.div` tokens per beat (default 4 => 16ths)
- **Swing**: `Song.swing` applies alternating long/short step durations while preserving average tempo
- **Playback modes** (`Track.playback`):
  - `gate` (default): voices are clipped per step; ties extend hold time
  - `mono`: stop the previous voice before starting a new one
  - `one_shot`: allow overlapping voices
- **Pattern semantics implemented in playback**:
  - pitch offsets (`+N`/`-N`)
  - velocity (`vN`) and accent (`X`)
  - probability (`?…`)
  - ratchets (`{N}`)
  - chords (polyphony via multiple events in a step)
  - gate length (`=…`) + ties (`_`) in gate mode
- **Delay**: per-track tempo-synced feedback delay

## Making changes safely

This repo follows strict TDD (Red → Green → Refactor). Keep changes small, write the failing test first, and prefer pure helpers (parser/compile/timing) over integration-heavy tests.



