# Live Playing View (Classic REPL)

The classic REPL (`--repl`) has a lightweight “live view” that can be toggled with `:live`. It prints a status header and (when snapshots are available) a small per-track grid.

This feature is intended for **REPL mode** only.

## Toggle

- `:live` — show current state
- `:live on` — enable
- `:live off` — disable

## What it prints (current)

When enabled, a background ticker periodically prints:

```text
[live] status:playing
Tracks:
1 Kick   | x . x . x . x . x . x . x . x .
2 Snare  | . . . . x . . . . . . . x . . .
```

The playhead position is highlighted using ANSI color escape codes.

## Implementation notes

- `:live` is wired in `src/repl/mod.rs` (`handle_meta` + `ensure_live_ticker`).
- Snapshots come from `audio::snapshot_live_state`.
- The REPL installs a `rustyline` external printer so background output doesn’t disrupt the input line.

## TUI note

The default TUI already renders a continuously-updating tracker grid. `:live` is not designed for the TUI and can corrupt the full-screen display if invoked there.



