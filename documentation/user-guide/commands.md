# Command Reference

## Quick Reference

| Type | Action |
|------|--------|
| `go` / `play` | ▶ Start playback |
| `.` / `stop` | ⏹ Stop playback |
| `120` / `bpm 120` | Set tempo |
| `+ kick` | Add track |
| `- kick` | Remove track |
| `list` / `ls` | Show all tracks |
| `kick x...x...` | Set pattern |
| `kick ~ 909/kick` | Set sample (fuzzy search) |
| `kick -3db` | Set gain |
| `kick mute` | Mute track |
| `kick unmute` | Unmute track |
| `kick solo` | Toggle solo |
| `kick delay on` | Enable delay |
| `kick.fill x.x.` | Set variation |
| `kick > fill` | Switch to variation |
| `kick gen euclid(5,16)` | Generate pattern |
| `kick ai "funky"` | AI pattern suggestions |
| `save song.yaml` | Save song |
| `open song.yaml` | Load song |
| `?` / `:help` | Show help |
| `:live on` | Enable live view |
| `:q` | Quit |

---

## Transport

| Command | Description |
|---------|-------------|
| `go` or `play` | Start playback |
| `.` or `stop` | Stop playback |
| `120` or `bpm 120` | Set tempo to 120 BPM |
| `swing <percent>` | Set swing (0-100) |

---

## Track Management

| Command | Description |
|---------|-------------|
| `+ name` or `track name` | Add a new track |
| `- name` or `remove name` | Remove track |
| `list` or `ls` | Show all tracks with patterns |

Track names must be single words (no spaces). Examples: `kick`, `snare`, `hihat`, `hi-hat`.

---

## Track Commands

All track commands start with the track name, followed by the action:

### Pattern

```
kick x...x...x...x...
snare ..x...x...x...x.
hihat xxxxxxxxxxxxxxxx
```

Patterns use `x` for hits and `.` for rests. Spaces are ignored for readability:

```
kick x... x... x... x...
```

### Sample

Use `~` to set a sample with fuzzy matching:

```
kick ~ 909/kick
snare ~ snare
hihat ~ hat
```

Tab completion shows matching samples as you type.

### Gain

```
kick -3db
snare +2db
hihat -6db
```

### Mute / Solo

```
kick mute
kick unmute
snare solo
```

### Delay

```
kick delay on
kick delay off
kick delay 1/8 0.4 0.3    # time, feedback, mix
```

---

## Variations

Store multiple patterns per track and switch between them:

```
kick x...x...x...x...       # main pattern
kick.fill x.x.x.x.x.x.x.x.  # define "fill" variation
kick > fill                  # switch to fill
kick > main                  # back to main
```

---

## Pattern Generation

### Scripted (Rhai)

```
kick gen euclid(5,16)
kick gen random(0.3,42)
kick gen fill(16)
```

Built-in generators:
- `euclid(k, n)` — Euclidean rhythm (k hits in n steps)
- `random(density, seed)` — Random pattern (density 0.0-1.0)
- `fill(length)` — Drum fill pattern
- `invert(pattern)` — Swap hits and rests
- `rotate(pattern, n)` — Rotate pattern by n steps

### AI Generation

```
kick ai "funky"
snare ai "breakbeat"
```

Requires Ollama running locally (`http://localhost:11434`).

---

## Files

| Command | Description |
|---------|-------------|
| `save song.yaml` | Save song to YAML |
| `open song.yaml` | Load song from YAML |

---

## Meta Commands

| Command | Description |
|---------|-------------|
| `?` or `:help` | Show help |
| `:live on` | Enable live playhead view |
| `:live off` | Disable live view |
| `:q` or `:quit` | Exit |

---

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

---

## Legacy Commands

These older command styles still work for backwards compatibility:

| Old Style | New Style |
|-----------|-----------|
| `pattern kick "x..."` | `kick x...` |
| `sample kick "path"` | `kick ~ path` |
| `mute kick` | `kick mute` |
| `gain kick -3` | `kick -3db` |
| `bpm 120` | `120` |

---

## Tips

- **Tab completion**: Works for track names, sample paths, and commands
- **Fuzzy sample search**: Type part of a sample name after `~` and Tab
- **Pattern spacing**: Spaces in patterns are ignored for readability
- **Quick tempo**: Just type a number (e.g., `120`) to set BPM
