# Pattern Notation Guide

This document defines the visual pattern notation for Groove CLI. It covers what works today and the full planned notation so authored patterns remain forward‑compatible. Where useful, it draws on established practice from trackers (Renoise/FT2), Elektron trig conditions, and live‑coding mini‑notations (TidalCycles) while keeping the syntax compact and readable in a REPL.

Status key:

- [✓] Implemented in current engine
- [▲] Planned (syntax stable; parsed/ignored today)
- [⧗] Under consideration (subject to change)

## Basics

- Steps advance at `track.div` steps per beat; engine tempo is `song.bpm`.
- A pattern is a string; whitespace is ignored except inside brackets/parentheses.
- Length equals the count of step tokens (whitespace removed). Patterns loop.
- Bar separators (`|`) and thin spacing are for readability and are ignored. [▲]

Symbols (hits vs rests):

- `x` — hit (alias: `X`, `*`, `1`) [✓]
- `.` — rest/silence [✓]
- `_` — tie/sustain previous note through this step (no retrigger) [▲]

Examples:

- `x... x... x... x...` — four-on-the-floor [✓]
- `..x. ..x. ..x. ..x.` — off‑beat snare [✓]

## Per‑step modifiers

Attach modifiers immediately to a hit. Order of modifiers does not matter; use brackets if adding multiple key/value pairs.

- Pitch transpose: `+n` or `-n` semitones, e.g., `x+2`, `x-5` [▲]
- Probability: `?p` where `p` is `0–1` or `%` form, e.g., `x?0.35`, `x?35%` [▲]
- Velocity/accent: uppercase `X` for accent; or `vNN` (0–127), e.g., `xv96` [▲]
- Ratchet (sub‑repeats within the step): `{N}` e.g., `x{3}` = 3 quick retrigs in the step [▲]
- Micro‑timing nudge: `@±T` e.g., `x@+5ms`, `x@-8%` (ms or percent of step) [▲]
- Length override (gate/hold): `=T` e.g., `x=3/4` to hold 75% of the step; combines with ties `_` [▲]
- Humanize (randomize): `~spec` e.g., `x~5ms`, `x~3vel` [⧗]

Multiple modifiers example: `x+7?v50%{2}@-10ms=1/2` [▲]

## Grouping and bars

- Spaces cluster visually; no timing effect. [✓]
- `|` may separate bars for readability: `x...|x...|x...|x...` [▲]
- Parentheses group steps for repetition or chords (see below). [▲]

## Repeats and transforms

- Group repeat: `(pattern)*N` duplicates the enclosed steps N times. Example: `(x.)*4` → `x.x.x.x.` [▲]
- Speed scaling per group: `(pattern)/N` slow; `(pattern)*N!` fast density (Tidal‑style inspiration). Example: `(x.)/2` spans twice the time [⧗]

## Chords and pitched samples

For pitched samples (e.g., a synth note), a single step can trigger multiple transpositions:

- Chord as offsets: `x+(0,4,7)` (unison, +4, +7 semitones) [▲]
- Inline stacked hits: `(x x+4 x+7)` same step duration, simultaneous [▲]

## Conditional and cycle‑aware hits

Cycle = one full pass of the pattern.

- K of N cycles: `x@1/2` (only on the first of every 2 cycles) [▲]
- Every Nth occurrence: `x@%4` (on steps where global step index % 4 == 0) [⧗]

## Per‑step parameter locks (FX)

Attach bracketed key/value pairs to a hit to override track parameters on that event.

- Example: `x[delay.time=1/8, delay.mix=0.25]` [▲]
- Shorthand booleans: `x[delay.on]` or `x[rev.off]` [▲]

## Comments

- Use `#` to comment to end of line in multi‑line patterns (YAML). [▲]

## Escaping and embedding

- REPL: standard: wrap patterns in double quotes: `pattern 1 "x... x... x... x..."` [✓]
- REPL: chaining:  `pattern("x... x... x... x...")`[✓]
- YAML (current sample): `pattern: !Visual x...` [✓]

## Grammar (EBNF‑style)

This grammar reflects the planned full notation; the current engine accepts hits/rests and ignores unknown tokens safely.

```
pattern      := (bar | ws)* (group | step | bar | ws)+
bar          := '|'
group        := '(' (group | step)+ ')' group_mod*
group_mod    := '*' INT            # repeat N times
              | '/' INT            # stretch duration by N
step         := rest | note
rest         := '.' | '_'
note         := hit modifiers*
hit          := 'x' | 'X' | '*' | '1'
modifiers    := pitch | prob | vel | ratchet | nudge | gate | human | chord | plock | cycle
pitch        := ('+'|'-') INT
prob         := '?' ( FLOAT | INT '%' )
vel          := 'v' INT            # 0..127; X implies accent preset
ratchet      := '{' INT '}'
nudge        := '@' ('+'|'-')? ( INT 'ms' | INT '%' )
gate         := '=' ( FRACTION | FLOAT )
human        := '~' ( INT 'ms' | INT 'vel' )
chord        := '+(' INT (',' INT)* ')'
plock        := '[' kv (',' kv)* ']'
kv           := KEY '=' VALUE      # keys are namespaced like delay.time
cycle        := '@' INT '/' INT    # K/N cycles
ws           := /\s+/
```

Notes:

- Unknown modifiers are ignored gracefully to preserve forward compatibility.
- Float/time parsing supports `1/4`, `3/8`, `250ms`, and percentages for step‑relative durations.

## Design influences

- Trackers (Renoise/FT2): step grids, ties, per‑step parameters.
- Elektron trig conditions and retrigs: probability, cycle conditions, ratchets.
- TidalCycles mini‑notation: group repetition and time scaling concepts.

## Capability matrix

- Today [v0.x]:
  - Hits: `x` (aliases `X`, `*`, `1`) [✓]
  - Rests: `.` [✓]
  - Whitespace ignored; patterns loop [✓]

- Near‑term (parser work):
  - Pitch `+/-n`, ties `_`, ratchets `{N}`, probability `?p`, velocity `vNN`, bar `|` [▲]
  - Per‑step FX locks `[key=val,...]`, cycle conditions `@K/N`, nudge `@±T`, gate `=T` [▲]

- Coded generators (separate feature):
  - `euclid(k, n)` Euclidean rhythms; returns a sequence of hits with optional accents [▲]
  - User‑defined functions in an embedded language (see `documentation/features/full-spec.md`). [▲]

## Examples

- Four bars of kick with snare backbeats:
  - `x...|x...|x...|x...`
  - `..x.|..x.|..x.|..x.`

- Hi‑hat with ratchets and probability accents:
  - `x{2}. x?30% x{3}. x?20%`

- Pitched synth arpeggio (C, E, G, octave):
  - `x x+4 x+7 x+12`

- Snare with delay only on accents:
  - `xv80 xv110[delay.on, delay.time=1/8] xv80 xv110[delay.on]`

- Conditional fill every 4th cycle:
  - `.... .... .... x@1/4{4}`

## Authoring tips

- Keep lines to 64–80 visible characters for readability.
- Use bars `|` and spacing to communicate structure to humans.
- Prefer explicit `div` per track rather than encoding tempo changes in patterns.

## Compatibility and migration

- Existing simple patterns remain valid. Additional characters are ignored by current versions, so you can start writing `x+2` or `x?35%` today without breaking playback; they will take effect once the parser ships.

See also:

- documentation/features/full-spec.md
