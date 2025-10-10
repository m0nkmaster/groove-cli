# Live Playing View

This document describes the lightweight “live playing view” that can be toggled from the REPL. It is intentionally minimal to keep the REPL fast and readable, and serves as a foundation for richer TUI work later.

## Toggle

- `:live on` — enable the live view header
- `:live off` — disable it
- `:live` — print current state

When enabled, the REPL prints a compact status line before each prompt showing tempo, steps, swing, and whether audio is currently playing. If playback is running, a per‑track grid is rendered with the current playhead position highlighted in green.

Example interaction:

```
> :live on
live view on
[live] bpm:120 steps:16 swing:0% status:stopped
> play
[play]
[live] bpm:120 steps:16 swing:0% status:playing
> stop
[stop]
[live] bpm:120 steps:16 swing:0% status:stopped
> :live off
live view off
>
```

## UI Examples

- Header (always when enabled)
```
[live] bpm:120 steps:16 swing:10% status:playing
```

- Header + Track Grid (implemented)

Playhead location is shown in green for each track.

```
[live] bpm:124 steps:16 swing:0% status:playing
Tracks:
1 Kick   | x . \x1b[32mx\x1b[0m . x . . . x . . . x . . .
2 Snare  | . . . . \x1b[32mx\x1b[0m . . . . . . . x . . .
3 Hat    | x . x . x . x . x . x . \x1b[32mx\x1b[0m . x .
```

## Notes for Development

- The current implementation avoids timers or background redraw loops in the REPL; the grid reflects the latest audio snapshot when the prompt appears.
- When `:live on` and playback is active, a lightweight background ticker prints the header + grid automatically whenever the playhead advances (about every 250ms). This keeps output readable and avoids constant spam.
- The `status:playing` indicator reflects the audio engine’s runtime state.
- Future enhancements can reuse this toggle to drive a Ratatui/TUI panel or structured redraw.
