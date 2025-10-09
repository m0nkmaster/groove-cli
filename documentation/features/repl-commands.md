# REPL Command Reference

This reference describes the current and planned REPL commands, syntax, semantics, and live-update behavior while audio is running.

## Principles

- Minimal, legible commands with immediate feedback.
- Safe, hot-swappable changes to the live object graph (song/tracks) while transport runs.

## Command Grammar

- Identifiers: `let name = ...` (planned)
- Calls: `name.method(args...)` (planned)
- Core commands (current scaffold):
  - `bpm <n>` — set global tempo (`u32`, 60..180 in v0.1)
  - `steps <n>` — set steps per bar (`u8`, 8..32 in v0.1)
  - `swing <percent>` — 0..100
  - `track "Name"` — create a new track appended to the list
  - `sample <track_idx> "path"` — set track sample (with autocomplete; see sample-autocomplete.md)
  - `pattern <track_idx> "visual"` — set visual pattern on track
  - `list` — print track summaries and FX states
  - `play` / `stop` — transport control (stubbed in scaffold)
  - `save "song.toml"` / `open "song.toml"` — persistence
  - `:help`, `:q` — meta commands

## Visual Patterns

- Syntax examples: `"x... x... x... x..."`, `"x+3"`, `"x@96"`, `"x!!!"`, `"x%35"`.
- Spacing groups steps visually (e.g., 4×4 grid).

## Live Update Behavior

- Design target: All commands mutate the in-memory object graph. The scheduler consumes deltas safely:
  - Tempo changes (`bpm`) apply next tick.
  - `pattern` changes for a track take effect from the next step boundary (or immediately if safe), without glitching.
  - FX parameter changes (delay on/time/fb/mix) apply atomically to the track’s effect node.
  - `mute`/`solo`/`gain` apply immediately.

## Planned Commands (v0.1 → v0.2)

- `delay <idx> on|off`
- `delay <idx> time "1/4" fb <0..1> mix <0..1>`
- `mute <idx> [on|off]`
- `solo <idx> [on|off]`
- `gain <idx> <db>`
- `remove <idx>`
- `meter [idx]`
- `quantise <idx> <grid>`

## Autocomplete

- Methods and file paths are autocompleted.
- Sample path autocomplete is detailed in `sample-autocomplete.md`.

