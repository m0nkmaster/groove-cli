# Groove CLI

A command-line groovebox with a powerful REPL for building patterns and playing samples. Music making for software engineers!

## Features

- **Pattern Sequencer** with velocity, chords, ratchets, probability, and micro-timing
- **Tempo-synced Delay** effect per track
- **Rhai Scripting** for generative patterns (`euclid`, `random`, `fill`, and more)
- **Pattern Variations** for live switching between arrangements
- **AI-powered Pattern Generation** via local LLM (Ollama)
- **Tab Completion** for sample paths and commands
- **Hot Reload** from YAML files with live playback updates
- **Live View** showing playhead position and track status

## Install

Prerequisites: Rust toolchain (`rustup`), audio backend supported by `rodio` (CoreAudio/macOS, WASAPI/Windows, ALSA/PulseAudio/Linux).

```bash
cargo build --release
./target/release/groove-cli
```

Or run directly:
```bash
cargo run --
```

## Quick Start

```
> track "Kick"
> sample 1 "samples/kits/harsh 909/Kick.wav"
> pattern 1 "x... x... x... x..."
> bpm 120
> play
```

Or chain commands:
```
> track("Kick").sample(1, "samples/909/kick.wav").pattern(1, "x...x...x...x...")
```

Enable live view:
```
> :live on
```

## Pattern Notation

| Syntax | Description |
|--------|-------------|
| `x` or `X` | Hit (X = accented) |
| `.` | Rest |
| `_` | Tie/sustain |
| `x+7` | Pitch up 7 semitones |
| `xv80` | Velocity 80 (0-127) |
| `x?50%` | 50% probability |
| `x{3}` | Ratchet (3 sub-hits) |
| `(x x+4 x+7)` | Chord |
| `x=3/4` | Gate length 75% |

Example: `x... xv60?50% x{2}. X`

## Scripted Patterns

Generate patterns with Rhai:
```
> gen 1 `euclid(5, 16)`
track 1 pattern: x..x.x..x.x..x..

> gen `random(0.3, 42)`
generated: x...x.....x..x..
```

Built-in generators: `euclid(k, n)`, `random(density, seed)`, `fill(length)`, `invert(pattern)`, `rotate(pattern, n)`

## Pattern Variations

Store multiple patterns per track and switch live:
```
> pattern 1.a "x...x...x...x..."
> pattern 1.b "x.x.x.x.x.x.x.x."
> var 1 b
track 1 switched to variation 'b'
```

## Effects

Per-track delay:
```
> delay 1 on
> delay 1 time 1/8
> delay 1 feedback 0.4
> delay 1 mix 0.3
```

## AI Pattern Generation

Generate patterns using a local LLM (requires Ollama):
```
> ai 1 "funky kick pattern"
Generating patterns for 'funky kick pattern'...
Suggestions:
  1) x..x..x.x...x...
  2) x...x.x...x.x...
  3) x.x...x...x.x...
```

## Commands Reference

See `documentation/user-guide/commands.md` for the full command list.

Key commands:
- `play` / `stop` - Transport control
- `bpm <n>` - Set tempo
- `track "Name"` - Add track
- `sample <idx> "path"` - Set sample
- `pattern <idx> "..."` - Set pattern
- `mute <idx>` / `solo <idx>` - Mix control
- `gain <idx> <db>` - Volume
- `gen <idx> \`script\`` - Generate pattern
- `var <idx> <name>` - Switch variation
- `save "file.yaml"` / `open "file.yaml"` - Persistence
- `:live on` - Enable live view
- `:help` - Show help

## Documentation

- [Quickstart Guide](documentation/user-guide/quickstart.md)
- [Command Reference](documentation/user-guide/commands.md)
- [Pattern Notation](documentation/user-guide/pattern-notation.md)

## License

MIT
