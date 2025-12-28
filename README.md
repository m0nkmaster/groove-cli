# Groove CLI

A terminal groovebox with a tracker-style TUI and a fast command language for building patterns and playing samples.

## Features

- **Default TUI** (Ratatui): live tracker grid + command line + message log
- **Classic REPL** (`--repl`): `rustyline` prompt with history + tab completion
- **Visual pattern DSL**: pitch (`+N/-N`), velocity (`vN`), probability (`?…`), ratchets (`{N}`), chords, gate/ties
- **Per-track delay**: tempo-synced feedback delay (time / feedback / mix)
- **Scripted generators (Rhai)**: `euclid`, `random`, `fill`, `invert`, `rotate`, `humanize`, …
- **AI pattern generation (optional)**: OpenAI Responses API (`OPENAI_API_KEY`)
- **Fuzzy sample selection**: `track ~ query` with Tab completion
- **YAML save/load**: `save …`, `open …`, `--open …`

## Install

Prereqs:
- Rust toolchain (`rustup`)
- A supported audio backend for `rodio` (CoreAudio/macOS, WASAPI/Windows, ALSA/PulseAudio/Linux)

Build:

```bash
cargo build --release
```

Run:

```bash
./target/release/groove-cli
```

Or run in dev:

```bash
cargo run --
```

### Flags

- `--open <file.yaml>` / `-o <file.yaml>`: open a song on startup
- `--repl`: start in classic REPL mode (default is the TUI)
- `--quiet` / `-q`: reduce the REPL startup banner (REPL mode)

## Quick start

These commands work in both the TUI and the REPL:

```text
+ kick
kick ~ 909/kick
kick x...x...x...x...

+ snare
snare ~ snare
snare ....x.......x...

go
```

Stop:

```text
.
```

## AI setup (optional)

Set env vars (or put them in a `.env` file in the repo root):

```bash
export OPENAI_API_KEY="..."
# optional
export OPENAI_MODEL="gpt-5.2"
```

Use:

```text
kick ai "four on the floor"   # applies to the track
ai kick "funky breakbeat"     # prints suggestions
```

## Documentation

- [Quickstart Guide](documentation/user-guide/quickstart.md)
- [Command Reference](documentation/user-guide/commands.md)
- [Pattern Notation](documentation/user-guide/pattern-notation.md)
- [Development Guide](documentation/development/DEVELOPMENT.md)

## License

MIT



