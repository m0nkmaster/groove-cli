# TOML Schema

This document defines the serialization format for `Song` objects. The schema is designed for stability and forward compatibility.

## Top-Level

```toml
bpm = 120        # u32
steps = 16       # u8
swing = 0        # u8 percent

[[tracks]]       # array of Track
...
```

## Track

Fields correspond to the Rust `Track` struct.

```toml
[[tracks]]
name = "Kick"              # string
sample = "samples/909/kick.wav"  # string | null
delay = { on=false, time="1/4", feedback=0.35, mix=0.25 }
pattern = { Visual = "x... x... x... x..." }  # enum encoding
mute = false
solo = false
gain_db = 0.0
```

## Delay

```toml
delay = { on = false, time = "1/4", feedback = 0.35, mix = 0.25 }
```

- `time`: musical division as string (e.g., `"1/8"`, `"1/4T"`).
- `feedback`: 0..1
- `mix`: 0..1

## Pattern Enum Encoding

`Pattern` currently has a single variant: `Visual(String)`.

TOML encodes Rust enums using an inline table with the variant name as the key and the value as the variant data:

```toml
pattern = { Visual = "x... x... x... x..." }
```

Future variants (e.g., `Coded`) should add new keys:

```toml
pattern = { Coded = { name = "euclid", args = [11, 16] } }
```

## Compatibility Policy

- New optional fields may be added with defaults.
- Unknown fields should be ignored by readers when possible.
- Dropping or renaming fields is a breaking change; prefer additive evolution.

## Example

See `song.toml` in the repository for a multi-track example.

