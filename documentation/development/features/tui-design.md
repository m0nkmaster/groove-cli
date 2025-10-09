# TUI Design

This document outlines the terminal UI (TUI) layout and live-updating behavior.

## Layout

```
┌──────────────────── CLI GROOVEBOX REPL ────────────────────┐
│ bpm: 120   steps: 16   swing: 0%   [Space] play/stop       │
│ >                                                        _ │
│                                                            │
│ Tracks                               Meters                │
│ 1 Kick  delay off                    |████▆▃▁|             │
│ 2 Snare delay 1/4 fb0.35 mix0.25     |▁▃▆█▇▅▂|             │
│ 3 Hat   delay off                    |▁▁▂▂▃▃▅|             │
│                                                            │
│ Playhead ▶ step 06/16                                       │
└────────────────────────────────────────────────────────────┘
```

## Behavior

- The REPL input sits at the top, with history and inline suggestions.
- Track list shows name and concise FX summary; updates instantly on changes.
- Meters display peak/RMS per track (simple fast decay in v0.1).
- Playhead shows current step and bar; ticks smoothly with clock.

## Controls

- Space: play/stop
- Arrow keys/PageUp/PageDown: navigate track list (selection for contextual actions)
- `:help`, `:doc <topic>`: inline help

## Rendering and Performance

- Ratatui-based renderer with a consistent frame rate (e.g., 30–60 Hz), decoupled from audio callback.
- UI subscribes to an observable state; rendering is a pure function of state.
  - The transport thread sends state deltas via a lock-free channel.
  - The UI applies deltas and re-renders; no heavy work in the audio thread.

## Extensibility

- Panels are modular (REPL, Inspector, Grid, Meters) to support future features like waveform previews.

