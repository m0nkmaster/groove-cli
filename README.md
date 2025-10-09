# groove-cli
CLI based REPL music sequencer

Status: early scaffold. Provides a minimal REPL and YAML save/load.

Quick start
- Build: `cargo build`
- Run: `cargo run --` or `cargo run -- -o song.yaml`

REPL commands
- `:help` – show help
- `track "Kick"` – add a track
- `sample 1 "samples/909/kick.wav"` – set sample on track 1
- `pattern 1 "x... x... x... x..."` – set visual pattern
- `bpm 120`, `steps 16`, `swing 0`
- `list` – list tracks
- `save "song.yaml"`, `open "song.yaml"`

Notes
- Audio engine and TUI are early prototypes; `play` performs a basic sample playback pass.
- See `documentation/features/full-spec.md` for the vision and spec.
- See `documentation/features/pattern-notation.md` for pattern notation (current and planned).
