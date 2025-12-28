# TUI Overview (Current)

This document describes the current TUI layout and interaction model. It’s not a design spec; it’s meant to help contributors orient themselves quickly.

## Layout

The TUI is rendered by `src/tui/mod.rs` and is split into four vertical sections:

- **Header**: transport state, BPM, track count, swing, playhead (UI step), and key hints
- **Tracker grid**: per-track pattern view with a moving playhead
- **Messages**: recent command output and console logs
- **Input**: a single-line command prompt with history and completion

## Interaction

- **Enter**: execute the current input line
- **Tab**: completion
  - common: complete `track ~ …` sample queries
  - also completes known commands and track names
- **Up/Down**: input history
- **Esc**: clear the current input line
- **Ctrl-C / Ctrl-D**: quit

## Rendering / output rules

The TUI is a full-screen redraw interface. Command handlers should avoid printing to stdout; route asynchronous logs through `crate::console` so they show up in the messages pane.


