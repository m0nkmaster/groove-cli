# Command Reference

Groove CLI has **one command language** used by both the **default TUI** and the **classic REPL** (`--repl`).

## Track identifiers

Many commands accept either:

- A **1-based track index** (e.g. `1`, `2`, …)
- A **track name** (case-insensitive, e.g. `kick`, `Snare`, `Hi-Hat`)

## Command styles

There are three equivalent ways to do most things:

### 1) Track-first (recommended)

Commands start with a track name:

```text
kick ~ 909/kick
kick x...x...x...x...
kick -3db
kick mute
kick delay 1/8 0.4 0.3
kick.fill x.x.x.x.x.x.x.x.
kick > fill
kick x... ~[linn snare class] -3db
```

### 2) Index-based (explicit)

Commands start with an action and a track id/name:

```text
sample kick 909/kick
pattern kick x...x...x...x...
gain kick -3
mute kick on
delay kick 1/8 0.4 0.3
```

### 3) Dot-chaining (sugar for index-based)

Calls can be chained with `.` and parentheses:

```text
track("Kick").sample(1, "samples/kits/harsh 909/Kick.wav").pattern(1, "x...x...x...x...")
```

This expands into the index-based commands: `track …`, `sample …`, `pattern …` (in order).

### Multiple commands per line (`;`)

You can run multiple commands on one line by separating them with `;`:

```text
+ kick snare; kick x...; snare ....x.......x...; go
```

Commands run **left-to-right** and stop at the **first error** (earlier changes are kept).

---

## Global / transport

- **Play**: `go` or `play`
- **Stop**: `.` or `stop`
- **Set BPM**: type a number (e.g. `140`) or `bpm 140`
- **Set swing**: `swing <percent>` (0–100)
- **Set steps (UI bar length)**: `steps <number>`

---

## Track management

- **Add track**:
  - `+ name [name...]` (atomic)
  - `track name`
- **Remove track**:
  - `- name` or `- 1`
  - `remove name` or `remove 1`
- **List tracks**: `list` or `ls`

Notes:
- Track names must be **single words** (no spaces).
- Remove accepts either index or name.

---

## Patterns

### Set main pattern

Track-first:

```text
kick x...x...x...x...
```

Index-based:

```text
pattern kick x...x...x...x...
```

If you want to include spaces, comments, or complex group syntax in interactive input, prefer the `pattern … "…"` form with quotes:

```text
pattern kick "x... x... x... x..."
```

### Variations

- **Define** a variation: `track.var <pattern>`

```text
kick.fill x.x.x.x.x.x.x.x.
```

- **Switch** variation: `track > var`

```text
kick > fill
kick > main
```

- **List / switch (index-based)**: `var <track> [name]`

```text
var kick
var kick fill
```

### Generate (Rhai)

Track-first:

```text
kick gen euclid(5,16)
```

Index-based:

```text
gen kick euclid(5,16)
```

Preview only (prints without applying):

```text
gen "euclid(5,16)"
```

### Progressions (chords)

Generate a chord progression as a **note/chord pattern** (major/minor triads):

```text
prog synth "C Am F G"        # uses the current song steps (default 16)
prog synth "C Am F G" 64     # total steps (chords are repeated to fill)
```

### Inspect resolved notes

Print the active pattern with notes resolved using the track’s root note:

```text
notes synth
```

---

## Samples

### Set sample (fuzzy)

Track-first:

```text
kick ~ 909/kick
```

Index-based:

```text
sample kick 909/kick
```

Tab completion works best with the track-first form: `kick ~ <Tab>`.

Multi-word fuzzy search is supported:

```text
kick ~ linn snare class
```

If you want to keep chaining more actions after the sample, use brackets or quotes:

```text
kick ~[linn snare class] -3db
kick ~ "linn snare class" -3db
```

In the **TUI**, when multiple matches exist, pressing **Tab cycles** through matches and inserts the current selection into the command line.

### Root note (pitch) detection / override

When you set a sample, Groove will try to detect its **root note** (shown in the TUI `NOTE` column). Note tokens (like `c d# eb`) depend on having a root note.

- Re-run detection: `analyze <track>`

```text
analyze synth
```

- Override manually: `root <track> <note>`

```text
root synth c4
root synth db3
```

### List/search samples

- **REPL**: `samples [filter]` lists matching samples grouped by directory
- **TUI**: typing `samples …` shows a short match list (search helper)

### Preview a sample (one-shot)

```text
preview 909/kick
preview samples/kits/harsh 909/Kick.wav
```

### Browse (REPL only)

The interactive browser is **only available in `--repl` mode**:

```text
browse samples
```

---

## Mix controls

### Gain

Track-first (supports `db` suffix):

```text
kick -3db
snare +2db
```

Index-based (number only; value is in dB):

```text
gain kick -3
```

### Mute / solo

Track-first:

```text
kick mute
kick unmute
kick solo        # toggles
```

Index-based:

```text
mute kick         # toggles
mute kick on
mute kick off
solo kick         # toggles
solo kick on
solo kick off
```

---

## Delay (per track)

Track-first:

```text
snare delay on
snare delay 1/8 0.4 0.3    # time, feedback (0..1), mix (0..1)
snare delay off
```

Index-based:

```text
delay snare on
delay snare 1/8 0.4 0.3
delay snare time "1/8" fb 0.4 mix 0.3
```

---

## Playback + timing (per track)

- **Playback mode**: `playback <track> <mode>`

Modes:
- `gate` (default): hit is clipped to a step (ties extend it)
- `mono`: stops the previous voice before starting a new one
- `one_shot`: overlapping voices allowed, sample plays out

```text
playback kick gate
playback kick mono
playback kick one_shot
```

- **Division (tokens per beat)**: `div <track> <1..64>`

```text
div kick 4     # 16ths at the current BPM
div kick 8     # 32nds
```

---

## AI

Apply directly to a track (uses the first suggestion):

```text
kick ai "four on the floor"
```

Generate suggestions (requires `OPENAI_API_KEY`):

```text
ai kick "funky breakbeat"
```

Optional env vars:
- `OPENAI_API_KEY`
- `OPENAI_MODEL` (default: `gpt-5.2`)

---

## Files

- **Save**: `save <file.yaml>`
- **Open**: `open <file.yaml>`

---

## Meta / UI

- **Help**: `?` or `:help`
- **Quit**:
  - REPL: `:q`, `:quit`, `:exit` (or Ctrl-D)
  - TUI: `:q`, `:quit`, `quit`, `exit` (or Ctrl-C)
- **REPL live view (REPL only)**:
  - `:live` / `:live on` / `:live off`
- **Clear screen (REPL)**: `clear`



