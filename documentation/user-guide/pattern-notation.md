# Pattern Notation Guide

This document defines the visual pattern notation for Groove CLI.

**Status key:**
- ✅ Implemented and working
- 🔜 Parsed but not yet applied in audio
- 💭 Under consideration

## Basics

Patterns are strings where each character represents one step. The sequencer plays at `track.div` steps per beat (default 4 = 16th notes) at `song.bpm` tempo.

| Symbol | Description | Status |
|--------|-------------|--------|
| `x` | Hit | ✅ |
| `X` | Accented hit (louder) | ✅ |
| `.` | Rest/silence | ✅ |
| `_` | Tie/sustain (extends previous note) | ✅ |
| `\|` | Bar separator (visual only) | ✅ |
| `#` | Comment to end of line | ✅ |

**Examples:**
```
x... x... x... x...     # Four-on-the-floor kick
..x. ..x. ..x. ..x.     # Backbeat snare
x.x. x.x. x.x. x.x.     # 8th note hi-hat
```

## Per-Step Modifiers

Attach modifiers directly after a hit. Order doesn't matter.

### Pitch Transpose ✅
```
x+7    # Up 7 semitones
x-5    # Down 5 semitones
x+12   # Up one octave
```

### Velocity ✅
```
xv80   # Velocity 80 (0-127)
xv127  # Maximum velocity
X      # Accent (preset loud velocity)
```

### Probability ✅
```
x?50%   # 50% chance to trigger
x?0.25  # 25% chance (decimal form)
x?75%   # 75% chance
```

### Ratchets (Sub-Repeats) ✅
```
x{2}    # 2 rapid hits within the step
x{3}    # Triplet roll
x{4}    # 4 quick hits (32nd notes at div=4)
```

### Gate Length ✅
```
x=1/2   # Hold for 50% of step duration
x=3/4   # Hold for 75%
x=1/4   # Short staccato
```

### Micro-timing Nudge 🔜
```
x@+5ms   # Trigger 5ms late
x@-10ms  # Trigger 10ms early
x@+5%    # Nudge by 5% of step duration
```

### Cycle Conditions 🔜
```
x@1/4   # Only trigger on cycle 1 of every 4
x@3/4   # Only on cycle 3 of 4
```

### Per-Step Parameter Locks 🔜
```
x[delay.on]              # Enable delay for this hit only
x[delay.time=1/8]        # Override delay time
x[delay.mix=0.5]         # Override delay mix
```

### Combined Modifiers
```
x+7?50%v80{2}   # Up 7 semitones, 50% probability, velocity 80, double hit
X=3/4           # Accented with 75% gate
```

## Chords ✅

Multiple notes on the same step:

### Inline chord notation
```
(x x+4 x+7)     # Major chord (root, major 3rd, 5th)
(x x+3 x+7)     # Minor chord
(x x+4 x+7 x+12) # Major with octave
```

### Shorthand chord offsets
```
x+(0,4,7)       # Same as (x x+4 x+7)
x+(0,3,7)       # Minor chord
```

Each note in a chord can have its own velocity:
```
(xv100 x+4v80 x+7v60)   # Root loudest, decreasing up
```

## Groups and Repeats ✅

### Repeat groups
```
(x.)*4          # Expands to: x.x.x.x.
(x..)*3         # Expands to: x..x..x..
(x x+4)*2       # Expands to: x x+4 x x+4
```

## Ties and Sustain ✅

Use `_` to extend a note through following steps (no retrigger):
```
x___....        # Hit on step 1, sustain through steps 2-4
x_x_x_x_        # Alternating hits with sustain
```

In `gate` playback mode, ties determine how long the sample plays before being cut.

## Comments ✅

In multi-line patterns (YAML files), use `#` for comments:
```yaml
pattern: !Visual |
  x... x... x... x...  # kick pattern
  # this line is ignored
```

## Whitespace

Spaces and tabs are ignored—use them freely for visual grouping:
```
x . . . | x . . . | x . . . | x . . .
```

## Full Grammar (EBNF)

```
pattern     := (step | group | bar | comment | ws)*
bar         := '|'
comment     := '#' [^\n]* '\n'
group       := '(' (step | group)+ ')' repeat?
repeat      := '*' INT
step        := rest | hit modifiers*
rest        := '.' | '_'
hit         := 'x' | 'X' | '*' | '1'
modifiers   := pitch | prob | vel | ratchet | nudge | gate | chord | plock | cycle
pitch       := ('+' | '-') INT
prob        := '?' (FLOAT | INT '%')
vel         := 'v' INT                    # 0-127
ratchet     := '{' INT '}'
nudge       := '@' ('+' | '-')? (INT 'ms' | INT '%')
gate        := '=' (FRACTION | FLOAT)
chord       := '+(' INT (',' INT)* ')'
plock       := '[' kv (',' kv)* ']'
kv          := KEY ('=' VALUE)?
cycle       := '@' INT '/' INT
```

## Examples

### Basic drum pattern
```
x... x... x... x...     # Kick
.... x... .... x...     # Snare
x.x. x.x. x.x. x.x.     # Hi-hat
```

### Hi-hat with dynamics
```
xv60 X xv40 xv60 X xv40 xv60 X
```

### Generative hi-hat
```
x x?30% x x?50% x x?30% x x?50%
```

### Snare with ratchet fill
```
.... x... .... x{4}.
```

### Synth arpeggio
```
x x+4 x+7 x+12 x+7 x+4
```

### Chord progression
```
(x x+4 x+7)___ (x+5 x+9 x+12)___ (x+7 x+11 x+14)___
```

### Delay on accents only
```
xv80 xv110[delay.on] xv80 xv110[delay.on]
```

## Tips

- Keep patterns under 80 characters for readability
- Use `|` and spaces to show bar structure
- Set `div` per track rather than changing pattern length
- Use probability for generative variation
- Combine ratchets and velocity for expressive rolls

## See Also

- [Command Reference](commands.md) — All REPL commands
- [Quickstart](quickstart.md) — Tutorial introduction
