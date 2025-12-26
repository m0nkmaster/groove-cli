# Quickstart Guide

Get making beats in minutes with groove-cli.

## Launch the REPL

```bash
cargo run --
# or
./target/release/groove-cli
```

Optional: Open an existing song with live reload:
```bash
cargo run -- -o songs/song.yaml
```

You'll see:
```
CLI GROOVEBOX REPL — bpm: 120 steps: 16 swing: 0% repeat:on (type :help)
>
```

## Create Your First Beat

### 1. Add a kick track

```
> track "Kick"
added track Kick
```

### 2. Load a sample

```
> sample 1 "samples/kits/harsh 909/Kick.wav"
track 1 sample set
```

💡 **Tip:** Press Tab for sample path autocomplete!

### 3. Set a pattern

```
> pattern 1 "x... x... x... x..."
track 1 pattern set
```

Pattern notation: `x` = hit, `.` = rest

### 4. Play it!

```
> play
```

Enable the live view to see playhead position:
```
> :live on
```

### One-liner version

Chain commands for speed:
```
> track("Kick").sample(1, "samples/909/kick.wav").pattern(1, "x...x...x...x...")
```

## Add More Tracks

```
> track "Snare"
> sample 2 "samples/kits/harsh 909/Snare.wav"
> pattern 2 ".... x... .... x..."

> track "HiHat"
> sample 3 "samples/kits/harsh 909/Closed Hat.wav"
> pattern 3 "x.x. x.x. x.x. x.x."
```

## Adjust the Mix

```
> gain 3 -6.0        # Lower hi-hat volume
> mute 2             # Mute snare
> solo 1             # Solo kick only
```

## Add Some Flavor

### Velocity and accents
```
> pattern 3 "xv60 X xv40 x"   # X = accent, v60 = velocity 60
```

### Probability (generative feel)
```
> pattern 3 "x x?50% x x?30%"  # 50% and 30% chance hits
```

### Ratchets (rolls)
```
> pattern 3 "x... x{3}. x... x{2}."  # Rapid sub-hits
```

### Delay effect
```
> delay 3 on
> delay 3 time 1/8
> delay 3 feedback 0.3
> delay 3 mix 0.2
```

## Generate Patterns with Code

Use Rhai scripts for algorithmic patterns:

```
> gen 1 `euclid(5, 16)`
track 1 pattern: x..x.x..x.x..x..
```

Built-in generators:
- `euclid(k, n)` — Euclidean rhythms
- `random(density, seed)` — Random patterns
- `fill(length)` — Drum fills

## Pattern Variations

Store multiple patterns per track for live switching:

```
> pattern 1.a "x...x...x...x..."   # main groove
> pattern 1.b "x.x.x.x.x.x.x.x."   # busy variation
> var 1 b                          # switch to busy
> var 1 main                       # back to main
```

## Save Your Work

```
> save "songs/my-beat.yaml"
song saved

> open "songs/my-beat.yaml"
song loaded
```

## Useful Commands

| Command | What it does |
|---------|--------------|
| `list` | Show all tracks |
| `bpm 140` | Change tempo |
| `swing 25` | Add swing feel |
| `stop` | Stop playback |
| `:help` | Show all commands |
| `:live on` | Enable live view |
| `clear` | Clear terminal |

## Next Steps

- Read the [Command Reference](commands.md) for all options
- Study [Pattern Notation](pattern-notation.md) for advanced patterns
- Try the AI generator: `ai "funky breakbeat kick"`

Happy beat making! 🥁
