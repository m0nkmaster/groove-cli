# Command Parsing & Surfaces (Developer Notes)

This document describes how commands are parsed/executed today, and where to add new ones.

## One command engine, two frontends

- **TUI**: `src/tui/` reads keystrokes and submits the current line to `repl::handle_line_for_tui`.
- **REPL** (`--repl`): `src/repl/` reads lines via `rustyline` and calls the same `handle_line`.

All behavior described below lives in `src/repl/mod.rs`.

## Parsing pipeline (in order)

1. **Semicolon-separated commands**
   - If the line contains `;` (outside quotes/brackets/parentheses), it is split into multiple commands and executed left-to-right.
   - Execution stops at the first error (no rollback of already-applied earlier commands).

2. **Meta commands**
   - Lines starting with `:` are handled by `handle_meta` (e.g. `:help`, `:q`).
   - `?` is a help shortcut (prints `help_box()`).

3. **Dot-chaining syntax**
   - `parse_chained_commands` recognizes expressions like:
     - `track("Kick").sample(1, "samples/...").pattern(1, "x...")`
   - It rewrites them into a sequence of index-based commands (`track …`, `sample …`, `pattern …`) and executes them in order.

4. **New-style “quick” commands**
   - A bare number sets BPM (e.g. `140`).
   - `+ name [name...]` adds tracks (atomic).
   - `- name` removes a track.
   - `>` is an alias for `go` (play).
   - `<` is an alias for `go` (play).

5. **Track-first syntax**
   - If the first token matches an existing track name, `try_track_first_command` parses **one or more segments** (left-to-right) and applies them **atomically per line** (commit + `audio::reload_song` once).
     - patterns: `kick x...`
     - sample selection: `kick ~ query...`, `kick ~[multi word query]`, `kick ~ "multi word query"`
     - variation set/switch: `kick.fill …`, `kick > fill`
     - wildcard selectors: `* > chorus`, `*piano* bridge` (shorthand for `> bridge`)
     - per-track actions: `kick mute`, `kick unmute`, `kick solo`, `kick delay …`, `kick gen …`, `kick ai …`, `kick -3db`
     - chaining example: `kick x... ~[linn snare class] -3db`

   - Macros are expanded before parsing when the line is a single token matching a stored macro name (see `macro` / `unmacro`).

6. **Index-based commands**
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



