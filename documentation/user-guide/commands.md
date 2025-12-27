# Command Reference

## Meta Commands

| Command | Description |
|---------|-------------|
| `:help` | Show built-in help |
| `:q` / `:quit` / `:exit` | Exit the REPL |
| `:doc` | Print documentation location |
| `:live [on\|off]` | Toggle or show the live playing view |

## Song Commands

| Command | Description |
|---------|-------------|
| `bpm <n>` | Set tempo (e.g., `bpm 120`) |
| `steps <n>` | Set steps per bar (model only) |
| `swing <percent>` | Set swing (0-100, affects timing) |
| `list` | Print track list with settings |
| `save "file.yaml"` | Save song to YAML |
| `open "file.yaml"` | Load song from YAML |

## Transport

| Command | Description |
|---------|-------------|
| `play` | Start playback |
| `stop` | Stop playback |
| `clear` | Clear terminal live view region |

## Track Management

| Command | Description |
|---------|-------------|
| `track "Name"` | Add a new track |
| `remove <idx>` | Remove track by index (1-based) |
| `sample <idx> "path"` | Set sample (validates & resolves path) |
| `samples [filter]` | List available samples |
| `preview "path"` | Play sample without setting |
| `pattern <idx> "x..."` | Set visual pattern |
| `mute <idx> [on\|off]` | Toggle or set mute |
| `solo <idx> [on\|off]` | Toggle or set solo |
| `gain <idx> <db>` | Set gain in dB (e.g., `gain 1 -3.0`) |
| `playback <idx> <mode>` | Set mode: `gate`, `mono`, or `one_shot` |
| `div <idx> <n>` | Set timing division (default 4 = 16ths) |

### Sample Shortcuts

You don't need to type full paths—shortcuts are resolved automatically:

```
> sample 1 "kick"
track 1 sample: samples/kits/harsh 909/Kick.wav

> sample 2 "909/snare"
track 2 sample: samples/kits/harsh 909/Snare.wav
```

Browse available samples:
```
> samples
Available samples:

samples/kits/harsh 909:
  Kick.wav
  Snare.wav
  Closed Hat.wav
  ...

> samples hat
Samples matching 'hat':

samples/kits/harsh 909:
  Closed Hat.wav
```

Preview before setting:
```
> preview "snare"
▶ samples/kits/harsh 909/Snare.wav
```

If a sample isn't found, you'll get suggestions:
```
> sample 1 "kik"
sample not found: kik

Did you mean:
  samples/kits/harsh 909/Kick.wav
  samples/kits/harsh 909/Kick Long.wav

Tip: use `samples` to list available samples
```

## Pattern Variations

| Command | Description |
|---------|-------------|
| `pattern <idx>.<var> "..."` | Set a named variation (e.g., `pattern 1.a "x..."`) |
| `var <idx> [name]` | Switch to variation or show available |
| `var <idx> main` | Switch back to main pattern |

Examples:
```
> pattern 1.a "x...x...x...x..."    # main groove
> pattern 1.fill "x.x.x.x.x.x.x.x." # fill pattern
> var 1 fill                         # switch to fill
> var 1 main                         # back to main
> var 1                              # list variations
track 1 variations: [main], a, fill
```

## Delay Effect

| Command | Description |
|---------|-------------|
| `delay <idx> on` | Enable delay |
| `delay <idx> off` | Disable delay |
| `delay <idx> time <t>` | Set delay time (`1/4`, `1/8`, `100ms`) |
| `delay <idx> feedback <f>` | Set feedback (0.0-1.0) |
| `delay <idx> mix <m>` | Set wet/dry mix (0.0-1.0) |

Example:
```
> delay 1 on
> delay 1 time 1/8
> delay 1 feedback 0.4
> delay 1 mix 0.25
```

## Pattern Generation

| Command | Description |
|---------|-------------|
| `gen <idx> \`script\`` | Generate pattern from Rhai script |
| `gen \`script\`` | Preview generated pattern |

Built-in generators:
- `euclid(k, n)` — Euclidean rhythm (k hits in n steps)
- `random(density, seed)` — Random pattern (density 0.0-1.0)
- `fill(length)` — Drum fill pattern
- `repeat(pattern, n)` — Repeat pattern n times
- `invert(pattern)` — Swap hits and rests
- `rotate(pattern, n)` — Rotate pattern by n steps
- `humanize(pattern, amount)` — Add velocity variation

Examples:
```
> gen 1 `euclid(5, 16)`
track 1 pattern: x..x.x..x.x..x..

> gen `random(0.25, 42)`
generated: ....x...x.......

> gen 1 `invert("x...x...x...x...")`
track 1 pattern: .xxx.xxx.xxx.xxx
```

## AI Pattern Generation

| Command | Description |
|---------|-------------|
| `ai [idx] "description"` | Generate patterns using LLM |

Requires Ollama running locally (`http://localhost:11434`).

Examples:
```
> ai "funk kick"
Suggestions:
  1) x..x..x.x...x...
  2) x...x.x...x.x...

> ai 1 "techno hi-hat"
Suggestions:
  1) x.x.x.x.x.x.x.x.
  2) x.xxx.xxx.xxx.xx
```

## Chaining Syntax

Commands can be written with parentheses and chained with dots:

```
> track("Kick").sample(1, "samples/kick.wav").pattern(1, "x...x...")
```

This executes all commands in sequence on one line.

## Notes

- Paths with spaces must be wrapped in quotes
- Track indices are 1-based
- Visual patterns ignore whitespace (use spaces for readability)
- If any track is solo, all non-solo tracks are muted
- Tab completion works for sample paths and commands
