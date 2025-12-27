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
♪ groove — type ? for help
♪ 120 ⏹ ›
```

## Create Your First Beat

### 1. Add a kick track

```
♪ 120 ⏹ › + kick
  ✓ added kick
```

### 2. Load a sample

```
♪ 120 ⏹ › kick ~ 909/kick
  kick  🔊 samples/kits/harsh 909/Kick.wav
```

💡 **Tip:** Type `kick ~` then press Tab to browse samples!

### 3. Set a pattern

```
♪ 120 ⏹ › kick x...x...x...x...
  kick  ●···●···●···●···
```

Pattern notation: `x` = hit, `.` = rest

### 4. Play it!

```
♪ 120 ⏹ › go
  ▶ playing
```

Enable the live view to see playhead position:
```
♪ 120 ▶ › :live on
  👁 live view on
```

## Add More Tracks

```
♪ 120 ▶ › + snare
  ✓ added snare

♪ 120 ▶ › snare ~ snare
  snare  🔊 samples/kits/harsh 909/Snare.wav

♪ 120 ▶ › snare ....x.......x...
  snare  ····●·······●···

♪ 120 ▶ › + hihat
  ✓ added hihat

♪ 120 ▶ › hihat ~ hat
  hihat  🔊 samples/kits/harsh 909/Closed Hat.wav

♪ 120 ▶ › hihat x.x.x.x.x.x.x.x.
  hihat  ●·●·●·●·●·●·●·●·
```

## Adjust the Mix

```
♪ 120 ▶ › hihat -6db
  hihat  🎚 -6.0db

♪ 120 ▶ › snare mute
  snare  🔇 muted

♪ 120 ▶ › kick solo
  kick  🎤 solo
```

## Add Some Flavor

### Velocity and accents
```
♪ 120 ▶ › hihat xv60.X.xv40.x...
  hihat  ●·◉·●·●···
```
X = accent, v60 = velocity 60

### Probability (generative feel)
```
♪ 120 ▶ › hihat x.x?50%.x.x?30%
```
50% and 30% chance hits

### Ratchets (rolls)
```
♪ 120 ▶ › snare ....x{3}.......x
```
Rapid sub-hits

### Delay effect
```
♪ 120 ▶ › snare delay on
  snare  🔁 delay on

♪ 120 ▶ › snare delay 1/8 0.3 0.2
  snare  🔁 delay 1/8 fb:0.30 mix:0.20
```

## Generate Patterns with Code

Use Rhai scripts for algorithmic patterns:

```
♪ 120 ▶ › kick gen euclid(5,16)
  kick  🎲 ●··●·●··●·●··●··
```

Built-in generators:
- `euclid(k, n)` — Euclidean rhythms
- `random(density, seed)` — Random patterns
- `fill(length)` — Drum fills

## Pattern Variations

Store multiple patterns per track for live switching:

```
♪ 120 ▶ › kick.a x...x...x...x...
  kick.a  ●···●···●···●···

♪ 120 ▶ › kick.fill x.x.x.x.x.x.x.x.
  kick.fill  ●·●·●·●·●·●·●·●·

♪ 120 ▶ › kick > fill
  kick  → fill

♪ 120 ▶ › kick > main
  kick  → main
```

## Save Your Work

```
♪ 120 ▶ › save my-beat.yaml
  💾 saved my-beat.yaml

♪ 120 ▶ › open my-beat.yaml
  📂 opened my-beat.yaml
```

## Quick Reference

| Command | What it does |
|---------|--------------|
| `go` / `play` | Start playback |
| `.` / `stop` | Stop playback |
| `120` | Set tempo to 120 |
| `+ name` | Add track |
| `- name` | Remove track |
| `list` / `ls` | Show all tracks |
| `name x...` | Set pattern |
| `name ~ sample` | Set sample |
| `name -3db` | Set gain |
| `name mute` | Mute track |
| `name solo` | Toggle solo |
| `?` | Show help |
| `:live on` | Enable live view |
| `:q` | Quit |

## Next Steps

- Read the [Command Reference](commands.md) for all options
- Study [Pattern Notation](pattern-notation.md) for advanced patterns
- Try the AI generator: `kick ai "funky breakbeat"`

Happy beat making! 🥁
