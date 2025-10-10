# Quickstart

This guide walks you through creating a simple beat with groove-cli.

## Run the REPL

- `cargo run --` (or run the installed binary `groove-cli`)
- Optional: `cargo run -- -o songs/song.yaml` to open and watch an existing song.

You’ll see a prompt:

```
CLI GROOVEBOX REPL — bpm: 120 steps: 16 swing: 0% repeat:on (type :help)
>
```

## Create Your First Track

1) Add a track:
- `track "Kick"`

2) Pick a sample:
- `sample 1 "samples/kits/harsh 909/Kick Short.wav"`

3) Set a pattern (x = hit, . = rest):
- `pattern 1 "x... x... x... x..."`

*Tip:* You can combine these three steps on one line with command chaining:

- `track("Kick").sample(1, "samples/kits/harsh 909/Kick Short.wav").pattern(1, "x... x... x... x...")`

4) Tempo and transport:
- `bpm 120`
- `play`
- `stop`

Optional live view:
- `:live on` to show a compact status line and a per‑track grid while playing.
- `clear` to manually clear the live output region if needed.

## Save and Open Songs

- Save to YAML: `save "songs/song.yaml"`
- Open from YAML: `open "songs/song.yaml"`

If a `song.yaml` exists in your current directory (or you pass `-o <file>`), the app watches it and live‑reloads on change.

## Tips

- `list` prints your tracks with their settings and patterns.
- `mute 1` or `solo 1` to focus listening.
- `gain 1 -3.0` to trim a loud sample.
- Playback defaults to `gate`; switch to `one_shot` if you want tails to overlap, or `mono` for monophonic voices.
