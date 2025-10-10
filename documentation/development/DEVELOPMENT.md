# Development Guide

This document is for engineers working on groove-cli. It describes the architecture, coding conventions, and workflows.

## Project Layout

- `src/main.rs` – CLI entrypoint, file watching, REPL boot
- `src/repl/` – REPL loop and command handlers
- `src/audio.rs` – Simple step sequencer and audio playback
- `src/model/` – Data model: `Song`, `Track`, `Pattern`, `fx`
- `src/storage/` – YAML save/load
- `tests/` – Integration tests
- `documentation/` – User and development docs

## Architecture

- REPL runs on the main thread; audio on a background thread.
- `audio::play_song` spawns the transport thread, keeping a `Sender` in a global `OnceCell` for control messages (Stop/Update).
- `build_config(song)` converts the model into a runtime config; if any track is `solo`, non‑solo tracks are muted.
- The scheduler ticks per track using `div` (tokens per beat) and `bpm`. It queues short `rodio` sources at each hit.
- File watching uses `notify` for directory events + a lightweight polling fallback to catch atomic‑rename editors.
- The REPL installs a `rustyline` external printer and runs a lightweight ticker when `:live on` to redraw a small status header and live grid without disrupting the input line.

## Conventions

- Keep command handlers small and single‑purpose; parse args with `shlex` and validate early.
- Favor pure functions for transformations (e.g., model → config) to keep testability high.
- Avoid blocking the audio thread; do filesystem, YAML, and heavy work on the REPL/main thread.

## Testing

- Unit tests live next to code (`#[cfg(test)]`), e.g., `audio.rs` and `repl/mod.rs` have focused tests.
- Integration tests live in `tests/`. Example: `song_yaml_roundtrip.rs`.
- Run: `cargo test`

Suggested additions:
- Add tests for REPL parsing edge cases and error messages.
- Add tests for file‑watch polling debounce.

## Local Scripts

Common commands:
- Build: `cargo build` (add `--release` for optimized binary)
- Lint/format: `cargo fmt` and `cargo clippy` (optional if installed)
- Test: `cargo test`

## Future Work

- Apply `fx::Delay` in audio engine (simple feedback delay per track)
- Implement swing/steps in scheduler
- REPL autocompletion for `sample` paths (see `development/features/sample-autocomplete.md`)
- TUI for track visualization and meters
