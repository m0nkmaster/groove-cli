# REPL Command Reference

This reference describes the current and planned REPL commands, syntax, semantics, and live-update behavior while audio is running.

## Principles

- Minimal, legible commands with immediate feedback.
- Safe, hot-swappable changes to the live object graph (song/tracks) while transport runs.

## Command Grammar (current)

- Core commands:
  - `bpm <n>` — set global tempo (`u32`)
  - `steps <n>` — set steps per bar (model only; audio pending)
  - `swing <percent>` — 0..100 (model only; audio pending)
  - `track "Name"` — create a new track appended to the list
  - `sample <track_idx> "path"` — set track sample
  - `pattern <track_idx> "visual"` — set visual pattern on track
  - `div <track_idx> <tokens_per_beat>` — per‑track timing division (4 → 16th notes)
  - `gain <track_idx> <db>` — adjust level in decibels
  - `mute <track_idx> [on|off]` — toggle or set
  - `solo <track_idx> [on|off]` — toggle or set (solo overrides mute)
  - `remove <track_idx>` — delete a track
  - `list` — print track summaries and FX states
- Transport: `play` / `stop`
- Persistence: `save "song.yaml"` / `open "song.yaml"`
- Meta/UI: `:help`, `:q`, `:doc`, `:live [on|off]`, `clear`

## Visual Patterns

- Syntax examples: `"x... x... x... x..."`, `"x+3"`, `"x@96"`, `"x!!!"`, `"x%35"`.
- Spacing groups steps visually (e.g., 4×4 grid).
- For broader terminal UI examples and scenario mockups, see `documentation/features/cli-ui-gallery.md`.

## Live Update Behavior

- All commands mutate the in-memory song. The scheduler consumes deltas safely:
  - Tempo changes (`bpm`) apply progressively without stopping playback.
  - `pattern` changes take effect smoothly; playhead advances on the new pattern.
  - `mute`/`solo`/`gain` apply immediately; solo state mutes all non‑solo tracks.
  - Delay parameters (on/time/fb/mix) update the model; audio effect application is pending.

## Planned Additions

- `repeat on|off` at the song level
- `meter [idx]` with simple peak/RMS
- `quantise <idx> <grid>`

## Autocomplete

- Methods and file paths are autocompleted.
- Sample path autocomplete is detailed in `sample-autocomplete.md`.
