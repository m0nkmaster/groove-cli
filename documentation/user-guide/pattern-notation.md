# Pattern Notation

Groove CLI patterns are **strings** where each symbol represents one step. Track timing is controlled by `div` (**tokens per beat**) and the song `bpm`.

## Entering patterns (TUI / REPL)

The easiest form is track-first:

```text
kick x...x...x...x...
```

For patterns that include **spaces** or more complex **group syntax**, use `pattern … "…"`:

```text
pattern kick "x... x... x... x..."
```

## Basics

- **Hit**: `x` (also `1` and `*` are accepted as hits)
- **Accent**: `X` (louder hit)
- **Rest**: `.`
- **Bar separator**: `|` (ignored, visual only)
- **Comment**: `# ...` to end of line (ignored)

Examples:

```text
x... x... x... x...      # four on the floor
....x.......x...         # backbeat
x.x.x.x.x.x.x.x.          # 8ths
```

## Ties (sustain)

`_` ties a hit into following steps (no retrigger). Ties matter most in **gate** playback mode.

```text
x___.   # one hit, sustained across 3 ties
```

## Per-step modifiers (implemented)

Modifiers are written immediately after a hit (order doesn’t matter).

### Pitch (semitones)

```text
x+7   x-5   x+12
```

### Velocity (0–127)

```text
xv64
xv127
X        # accent (uses an “accent velocity” when no explicit vN is present)
Xv50     # explicit velocity wins over accent
```

### Probability

```text
x?50%     # percent form
x?0.25    # 0..1 form
```

### Ratchets (sub-hits)

`{N}` subdivides a single step into **N** evenly-spaced triggers.

```text
x{3}      # triplet-style roll in one step
x{4}
```

### Gate length

Gate shortens (or lengthens) the first step’s hold time in **gate** mode.

```text
x=1/2
x=3/4
x=0.25
```

Ties still extend the total hold:

```text
x=1/2__   # half-step first, then ties extend additional full steps
```

## Chords / polyphony

### Chord offsets (recommended for interactive input)

```text
x+(0,4,7)         # major triad
x+(0,3,7)         # minor triad
```

### Inline chord group

```text
(x x+4 x+7)
```

## Groups and repeats

Groups can be repeated with `*N`:

```text
(x.)*4     # expands to x.x.x.x.
```

## Known limitations

The parser recognizes some additional modifiers (nudges, cycle conditions, param locks), but **playback currently ignores them**. If you see syntax like `@-5ms`, `@1/4`, or `[delay.time=…]`, it will parse but won’t affect audio yet.



