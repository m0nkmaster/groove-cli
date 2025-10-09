# CLI UI Gallery: Scenario Mockups

This gallery shows example CLI UI layouts for common scenarios in GrooveCLI. Use these as reference visuals while building the REPL and TUI.

## Overview

- First‑Run Onboarding
- Command Help + Hints
- Success + Error Feedback
- Interactive Confirmation
- Fuzzy Sample Picker
- Progress + Long Task
- Track List + Meters
- REPL Editing (Multi‑line)
- Diff Preview Before Save
- Live Logs / Watch Mode

## First‑Run Onboarding

```
┌────────────────────────── GROOVECLI ──────────────────────────┐
│ Welcome! Create a song to get started.                        │
│                                                                │
│ Suggested next steps:                                          │
│   1) bpm 120          # set tempo                              │
│   2) steps 16         # set steps per bar                      │
│   3) track "Kick"     # make your first track                  │
│   4) sample 1 "samples/909/kick.wav"                           │
│   5) pattern 1 "x... x... x... x..."                          │
│                                                                │
│ Tips: Press Tab for autocomplete. Type :help for commands.     │
└────────────────────────────────────────────────────────────────┘
```

## Command Help + Hints

```
> :help pattern

pattern <track_idx> "visual" | pattern <track_idx> fn

Visual syntax examples:
  "x... x... x... x..."     hit on 1s
  "x@96 .. x! .."            velocity per note (0–127 or !/!!/!!!)
  "x+3 x-5"                  per-note pitch in semitones
  "x%40"                     probability (0–100)

Hints: Use spaces to group steps visually; 4×4 common.
```

## Success + Error Feedback

```
> bpm 999
! error: bpm out of range (60..180)
  hint: try `bpm 120`

> bpm 120
✓ bpm set to 120 (applies next tick)
```

## Interactive Confirmation

```
> open "song.yaml"
This will replace your current unsaved song. Continue? [y/N] y
✓ loaded song.yaml (3 tracks)
```

## Fuzzy Sample Picker

```
> sample 2 "samples/909/"
Pick a sample (type to filter, ↑/↓ to navigate, Enter to select):
  ▸ kick.wav           48k • 16‑bit • 00:00.50
    snare.wav          48k • 16‑bit • 00:00.35
    hat.wav            48k • 16‑bit • 00:00.20
  Filter: "k"
```

## Progress + Long Task

```
> save "song.yaml"
Saving… [██████████░░░░░░░░] 52%  (writing patterns)
✓ saved song.yaml
```

## Track List + Meters

```
┌──────────────────────── Tracks ────────────────────────┐  ┌── Meters ──┐
│ #  Name     FX Summary                                │  │  Mix  |█▆▃│
│ 1  Kick     delay off                                 │  │  1    |██▇│
│ 2  Snare    delay 1/4 fb0.35 mix0.25                  │  │  2    |▃█▅│
│ 3  Hat      delay off                                 │  │  3    |▁▂▃│
│                                                      ▶│  └───────────┘
│ Playhead: step 06/16                                   │
└────────────────────────────────────────────────────────┘
```

## REPL Editing (Multi‑line)

```
> let hh = track("Hat")
..   .sample("samples/909/hat.wav")
..   .pattern("x.x. x.x. x.x. x.x.")
..   .fx.delay(time="1/8", fb=0.25, mix=0.2)
✓ track added: Hat
```

## Diff Preview Before Save

```
> save "song.yaml" --preview
Changes:
  track[2].fx.delay: off → time=1/8 fb=0.25 mix=0.20
  bpm: 118 → 120
Write file? [y/N] y
✓ saved song.yaml
```

## Live Logs / Watch Mode

```
> meter
time     mix  kick snare hat
00:12.3  -3dB -6dB -9dB  -12dB
00:12.9  -3dB -6dB -8dB  -12dB
00:13.5  -2dB -5dB -8dB  -11dB
  (press q to exit)
```
