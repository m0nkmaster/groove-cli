# Command Parsing & Surfaces (Developer Notes)

This document describes how commands are parsed/executed today, and where to add new ones.

## One command engine, two frontends

- **TUI**: `src/tui/` reads keystrokes and submits the current line to `repl::handle_line_for_tui`.
- **REPL** (`--repl`): `src/repl/` reads lines via `rustyline` and calls the same `handle_line`.

All behavior described below lives in `src/repl/mod.rs`.

## Parsing pipeline (in order)

1. **Meta commands**
   - Lines starting with `:` are handled by `handle_meta` (e.g. `:help`, `:q`).
   - `?` is a help shortcut (prints `help_box()`).

2. **Dot-chaining syntax**
   - `parse_chained_commands` recognizes expressions like:
     - `track("Kick").sample(1, "samples/...").pattern(1, "x...")`
   - It rewrites them into a sequence of index-based commands (`track …`, `sample …`, `pattern …`) and executes them in order.

3. **New-style “quick” commands**
   - A bare number sets BPM (e.g. `140`).
   - `+ name` adds a track.
   - `- name` removes a track.

4. **Track-first syntax**
   - If the first token matches an existing track name, `try_track_first_command` handles:
     - patterns: `kick x...`
     - sample selection: `kick ~ query or path`
     - variation set/switch: `kick.fill …`, `kick > fill`
     - per-track actions: `kick mute`, `kick unmute`, `kick solo`, `kick delay …`, `kick gen …`, `kick ai …`, `kick -3db`

5. **Index-based commands**
   - Remaining lines are tokenized with `shlex` and executed via a `match` on the first token.
   - Many commands accept a “track id” argument that can be either a 1-based index or a track name (see `parse_track_index`).

## Adding a new command

- Add a new match arm in the index-based `match cmd.as_str()` block in `src/repl/mod.rs`.
- If it should be available as a track-first command, extend `try_track_first_command`.
- If it should be completable, update:
  - `src/repl/completer.rs` (`COMMANDS`, `TRACK_COMMANDS`, and/or TUI completions)
  - help text in `src/repl/style.rs` (`help_box`)

## TUI considerations

The TUI is a full-screen redraw UI. Avoid printing to stdout from command handlers (it will corrupt the screen).

If a feature needs to emit async output (logs, reload notices, etc.), route it through `crate::console` so the TUI can display it safely.



