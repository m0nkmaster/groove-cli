# Pattern Playback Status & Next Steps

The visual pattern parser supports a rich DSL, but playback only implements a subset (by design). This doc captures what is wired end-to-end today and what remains.

## Source of truth

- Parsing: `src/pattern/visual/`
- Compilation to runtime events: `src/audio/compile.rs`
- Scheduling + triggering: `src/audio.rs`

## Implemented in playback (end-to-end)

- **Hits/rests/ties**: `x` / `.` / `_`
- **Pitch**: `+N` / `-N` semitone offsets (implemented via playback speed)
- **Notes (by name)**: `c d# eb` (requires a track root note)
- **Velocity + accent**: `vN` and `X`
- **Chords / polyphony**
  - Inline chord groups `(x x+4 x+7)`
  - Chord offsets `x+(0,4,7)`
- **Ratchets**: `{N}` sub-hits within a single step
- **Probability**: `?…` (per-track deterministic RNG)
- **Gate**: `=…` (in `gate` playback mode), with ties extending hold duration

## Parsed but not implemented in playback

These are accepted by the parser, but currently discarded by `audio::compile` (no runtime effect):

- **Nudges**: `@±Nms` / `@±N%`
- **Cycle conditions**: `@h/d`
- **Param locks**: `[key=value, key2]`

## Next steps (recommended order)

1. **Decide**: either remove these modifiers from the user-facing DSL until implemented, or thread them through compilation with explicit “no-op” behavior documented.
2. **Carry through compile**: extend `CompiledEvent` to include nudge/cycle/param locks (or a generic “extra” field).
3. **Implement semantics in `audio.rs`**:
   - cycle gating
   - event timing offsets (nudges)
   - a safe, well-scoped param-lock application model (likely starting with delay params)



