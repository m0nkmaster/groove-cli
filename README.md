# groove-cli

CLI groovebox with a simple REPL for building patterns and playing samples.

Status: early but usable. You can add tracks, set patterns, load samples, and play/stop. Songs save to YAML.

## Install

- Prerequisites: Rust toolchain (`rustup`), audio backend supported by `rodio` (CoreAudio/macOS, WASAPI/Windows, ALSA/PulseAudio/Linux).

Build from source:
- `cargo build --release`
- Run with `cargo run --` or `./target/release/groove-cli`

## Quick Start

Run the REPL:
- `cargo run --`
- Optional: `cargo run -- -o songs/song.yaml` to open and watch an existing YAML.

In the REPL, try:
- `track "Kick"`
- `sample 1 "samples/kits/harsh 909/Kick Short.wav"`
- `pattern 1 "x... x... x... x..."`
- `bpm 120`
- `play` / `stop`

Type `:help` for the full built-in command list.

Watching files: If you open a YAML (or a `song.yaml` exists in CWD), groove-cli watches it for changes and live‑reloads playback.

## Documentation

- User Guide
  - Quickstart: `documentation/user-guide/quickstart.md`
  - Commands: `documentation/user-guide/commands.md`
  - Pattern notation: `documentation/user-guide/pattern-notation.md`
- Development
  - Overview: `documentation/development/DEVELOPMENT.md`
  - Feature specs: `documentation/development/features/`

## Features (current)

- REPL with commands for bpm/steps/swing, track add/remove, sample assignment, pattern set, mute/solo, gain, division, save/open.
- Simple audio sequencer (per‑track division, gain, mute; solo overrides mute).
- Hot reload from YAML file via filesystem watcher + polling fallback.

## Limitations / Roadmap

- Effects (e.g., delay) are modeled but not yet applied in audio.
- Swing and steps aren’t yet used by the audio engine.
- No TUI yet; just REPL.

Contributions welcome. See `documentation/development/DEVELOPMENT.md`.
