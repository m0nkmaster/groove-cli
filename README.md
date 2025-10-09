# groove-cli
CLI based REPL music sequencer

Status: early scaffold. Provides a minimal REPL and TOML save/load.

Quick start
- Build: `cargo build`
- Run: `cargo run --` or `cargo run -- -o examples/song.toml`

REPL commands
- `:help` – show help
- `track "Kick"` – add a track
- `sample 1 "samples/909/kick.wav"` – set sample on track 1
- `pattern 1 "x... x... x... x..."` – set visual pattern
- `bpm 120`, `steps 16`, `swing 0`
- `list` – list tracks
- `save "song.toml"`, `open "song.toml"`

Notes
- Audio engine and TUI are not implemented yet; `play`/`stop` are stubs.
- See `documentation/features/full-spec.md` for the vision and spec.
