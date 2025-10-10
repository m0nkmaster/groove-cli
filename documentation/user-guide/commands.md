# Command Reference

Meta
- `:help` – Show this help
- `:q` / `:quit` / `:exit` – Exit the REPL
- `:doc` – Print where to find docs locally
 - `:live [on|off]` – Toggle or show the live playing view

 Song
- `bpm <n>` – Set tempo (e.g., 120)
- `steps <n>` – Set steps per bar (model only; not used in timing yet)
- `swing <percent>` – Set swing (0..100, affects timing)
- `list` – Print track list and settings
- `save "song.yaml"` – Save current song to YAML
- `open "song.yaml"` – Load a song from YAML

Transport
- `play` – Start playback
- `stop` – Stop playback
 - `clear` – Clear the terminal output region used by live view

Tracks
- `track "Name"` – Add a new track
- `remove <idx>` – Remove a track by index (1‑based)
- `sample <idx> "path"` – Set sample file path
- `pattern <idx> "x..."` – Set visual pattern (`x`=hit, `.`=rest)
- `mute <idx> [on|off]` – Toggle or set mute
- `solo <idx> [on|off]` – Toggle or set solo (solo overrides mutes)
- `gain <idx> <db>` – Set gain in decibels (e.g., `-3.0`)
- `div <idx> <tokens_per_beat>` – Set timing division (default 4 → 16th notes)

Notes
- Paths may contain spaces; wrap in quotes.
- Visual patterns ignore whitespace; use spaces to group beats.
- If any track is solo, all non‑solo tracks are muted.
