# Groove CLI

A command-line groovebox with a powerful REPL for building patterns and playing samples. Music making for software engineers!

## Features

- **Pattern Sequencer** with velocity, chords, ratchets, probability, and micro-timing
- **Tempo-synced Delay** effect per track
- **Rhai Scripting** for generative patterns (`euclid`, `random`, `fill`, and more)
- **Pattern Variations** for live switching between arrangements
- **AI-powered Pattern Generation** via local LLM (Ollama)
- **Fuzzy Sample Search** with Tab completion
- **Hot Reload** from YAML files with live playback updates
- **Live View** showing playhead position and track status
- **Beautiful UI** with emoji feedback and styled output

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
♪ 120 ⏹ › + kick
  ✓ added kick

♪ 120 ⏹ › kick ~ 909/kick
  kick  🔊 samples/kits/harsh 909/Kick.wav

♪ 120 ⏹ › kick x...x...x...x...
  kick  ●···●···●···●···

♪ 120 ⏹ › go
  ▶ playing
```

## Command Examples

```
+ snare                    # add track
snare ~ 909/snare          # set sample (fuzzy match)
snare ..x...x...x...x.     # set pattern
snare -2db                 # set gain
snare mute                 # mute track
kick.fill x.x.x.x.x.x.x.x. # create variation
kick > fill                # switch to variation
kick gen euclid(5,16)      # generate pattern
120                        # set tempo
go                         # play
.                          # stop
list                       # show tracks
?                          # help
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
kick gen euclid(5,16)
  kick  🎲 ●··●·●··●·●··●··

kick gen random(0.3,42)
  kick  🎲 ●···●·····●··●··
```

Built-in generators: `euclid(k, n)`, `random(density, seed)`, `fill(length)`, `invert(pattern)`, `rotate(pattern, n)`

## Pattern Variations

Store multiple patterns per track and switch live:
```
kick.a x...x...x...x...
kick.b x.x.x.x.x.x.x.x.
kick > b
  kick  → b
```

## Effects

Per-track delay:
```
kick delay on
kick delay 1/8 0.4 0.3
  kick  🔁 delay 1/8 fb:0.40 mix:0.30
```

## AI Pattern Generation

Generate patterns using a local LLM (requires Ollama):
```
kick ai "funky"
  ✨ generating...
  kick  ✨ suggestions:
     1) ●··●··●·●···●···
     2) ●···●·●···●·●···
```

## Commands Reference

See `documentation/user-guide/commands.md` for the full command list.

Key commands:
- `go` / `play` — Start playback
- `.` / `stop` — Stop playback
- `120` — Set tempo (just type a number)
- `+ name` — Add track
- `- name` — Remove track
- `name x...` — Set pattern
- `name ~ sample` — Set sample
- `name -3db` — Set gain
- `name mute` / `unmute` / `solo` — Mix control
- `name.var x...` — Set variation
- `name > var` — Switch variation
- `name gen expr` — Generate pattern
- `save file.yaml` / `open file.yaml` — Persistence
- `:live on` — Enable live view
- `?` — Show help

## Documentation

- [Quickstart Guide](documentation/user-guide/quickstart.md)
- [Command Reference](documentation/user-guide/commands.md)
- [Pattern Notation](documentation/user-guide/pattern-notation.md)

## License

MIT
