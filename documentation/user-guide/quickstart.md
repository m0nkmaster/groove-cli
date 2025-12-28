# Quickstart

Groove CLI is a terminal groovebox: you create tracks, assign samples, type patterns, and hit play.

## Install & run

Build and run:

```bash
cargo build --release
./target/release/groove-cli
```

Or during development:

```bash
cargo run --
```

### Modes

- **Default: TUI** (tracker-style UI with a command line). This is what you get with no flags.
- **Classic REPL**: `groove-cli --repl` (line-based prompt with `rustyline`).

### Open a song on startup

```bash
groove-cli --open songs/song.yaml
# or
groove-cli -o songs/song.yaml
```

In **REPL mode**, if you start with `--open …` (or if `song.yaml` exists in the current directory) the app will watch the file and **reload audio** when it changes.

## Make your first beat (TUI or REPL)

All commands below work in both modes unless noted.

### 1) Add tracks

```text
+ kick
+ snare
+ hat
```

Track names are **single words** (no spaces). You can use `- name` to remove them later.

### 2) Pick samples (with Tab completion)

Use `~` for fuzzy sample selection:

```text
kick ~ 909/kick
snare ~ snare
hat ~ hat
```

- **Tab completion**: in the TUI, press **Tab** after `~` to see matches; in the REPL, Tab completion is provided by `rustyline`.
- **Paths with spaces**: `track ~ …` accepts paths with spaces without extra quoting.

### 3) Enter patterns

```text
kick x...x...x...x...
snare ....x.......x...
hat x.x.x.x.x.x.x.x.
```

### 4) Play / stop

```text
go
.          # stop (a single dot)
```

Aliases: `play` / `stop`.

### 5) Tempo + swing

```text
140        # typing a number sets BPM
swing 15   # 0..100 (%)
```

Optional: set the UI “bar length” (used for the TUI playhead display):

```text
steps 16
```

## Mix & performance

### Gain

Track-first (recommended):

```text
hat -6db
snare +2db
```

### Mute / solo

```text
hat mute
hat unmute
kick solo   # toggles
```

## Delay (per track)

```text
snare delay on
snare delay 1/8 0.40 0.25   # time, feedback (0..1), mix (0..1)
snare delay off
```

## Variations (live pattern switching)

Define a variation:

```text
kick.fill x.x.x.x.x.x.x.x.
```

Switch:

```text
kick > fill
kick > main
```

## Generate patterns with code (Rhai)

```text
kick gen euclid(5,16)
hat gen random(0.35, 42)
snare gen fill(16)
```

Built-ins include: `euclid(k,n)`, `random(density, seed)`, `fill(length)`, `repeat(pattern, n)`, `invert(pattern)`, `rotate(pattern, n)`, `humanize(pattern, amount)`.

## AI pattern generation (OpenAI)

Set env vars (or put them in a `.env` file):

```bash
export OPENAI_API_KEY="..."
# optional
export OPENAI_MODEL="gpt-5.2"
```

Apply a suggestion directly to a track:

```text
kick ai "four on the floor"
```

Or generate suggestions without applying:

```text
ai kick "funky breakbeat"
```

## Save / load

```text
save my-song.yaml
open my-song.yaml
```

## Next

- [Command Reference](commands.md)
- [Pattern Notation](pattern-notation.md)


