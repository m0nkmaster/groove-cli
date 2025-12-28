# Development Status

This is a “source-of-truth” snapshot of what `groove-cli` currently ships, plus the most important known gaps.

## Implemented (shipped)

### UI

- **Default TUI** (`src/tui/`): tracker grid + command line + messages panel.
- **Classic REPL** (`--repl`, `src/repl/`): `rustyline` prompt + history + tab completion.
- **Tab completion**
  - TUI uses `repl::completer::complete_for_tui`
  - REPL uses `rustyline` helper (`repl::completer::GrooveHelper`)

### Commands

- **Track-first syntax** (recommended): `kick x...`, `kick ~ query`, `kick delay …`, `kick.fill …`, `kick > fill`, `kick gen …`, `kick ai …`, `kick -3db`, etc.
- **Index-based syntax**: `pattern 1 …`, `sample 1 …`, `delay 1 …`, `gain 1 -3`, `playback 1 mono`, `div 1 8`, etc.
- **Dot-chaining syntax**: `track("Kick").sample(1, "...").pattern(1, "...")` (parsed into index-based commands).

### Audio engine

- **Background transport thread** with live updates via `audio::reload_song`.
- **Pattern playback** (implemented semantics):
  - pitch offsets (`+N` / `-N`)
  - velocity (`vN`) + accent (`X`)
  - probability (`?…`)
  - ratchets (`{N}`)
  - chords / polyphony
  - gate length (`=…`) and ties (`_`) in `gate` mode
- **Per-track delay**: tempo-synced feedback delay (time/fb/mix).
- **Per-track division**: `Track.div` tokens per beat (default 4).
- **Swing**: `Song.swing` affects step timing.
- **Playback modes**: `gate` (default), `mono`, `one_shot`.

### Persistence

- YAML save/load via `serde_yaml` (`save …`, `open …`, `--open …`).
- **REPL mode** supports file watching + polling fallback that reloads **audio** on file changes.

### Generators

- Rhai scripted generators: `euclid`, `random`, `fill`, `repeat`, `invert`, `rotate`, `humanize`.

### AI

- Pattern generation via the OpenAI Responses API (`OPENAI_API_KEY`, optional `OPENAI_MODEL`).

## Known gaps / limitations (real, current)

- **Parsed-but-ignored pattern fields**: the visual parser supports nudges, cycle conditions, and param locks, but `audio::compile` currently discards them (no runtime effect).
- **TUI + `:live` / `clear`**: the classic REPL live ticker (`:live`) and `clear` are intended for `--repl` mode; running them inside the TUI can corrupt the TUI display (stdout/ANSI output).
- **File watching**: only enabled in `--repl` mode, and currently only reloads the audio engine (the in-memory UI `Song` is not replaced).
- **AI step length**: AI context currently uses a fixed 16-step assumption (independent of `Song.steps`).
- **Interactive sample browser**: `browse` is REPL-oriented; the TUI intentionally steers users to `track ~ …` with tab completion.

## Next high-value improvements

- Thread nudges/cycle conditions/param locks through `audio::compile` and into playback (or explicitly remove them from the DSL until implemented).
- Make AI generation respect `Song.steps`.
- Add TUI-safe equivalents for REPL-only features (`:live`, `clear`, interactive browse) or hide/disable them in the TUI command layer.



