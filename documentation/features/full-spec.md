# GrooveCLI: vision and technical spec

**Repository**: [git@github.com](mailto:git@github.com):m0nkmaster/groove-cli.git

## Vision

A fast, fun, sample‑based groovebox for developers. You live‑code music in a REPL. You create a `song`, add `track` objects, then set properties and write patterns. The UI shows code, state, meters and a playhead. It is not a mini DAW. It is a programmable instrument.

## Principles

* Instant feedback with low latency
* Clear, minimal commands
* Terminal‑first visuals that are legible and playful
* Text files for songs and samples
* Modular core so we can swap audio engine later

## Terminal UX sketch

```
┌──────────────────────────── CLI GROOVEBOX REPL ───────────────────────┐
│ bpm: 120   steps: 16   swing: 0%   [Space] play/stop   [:help] help    │
│                                                                        │
│ > let s = song()
│ > let k = track("Kick").sample("samples/909/kick.wav")
│ > k.pattern("x... x... x... x...")
│ > let sn = track("Snare").sample("samples/909/snare.wav")
│ > sn.pattern(". . . .  x . . .  . . . .  x . . .").fx.delay(time="1/4", fb=0.35, mix=0.25)
│ > let hh = track("Hat").sample("samples/909/hat.wav")
│ > hh.pattern("x+2.x.x. x-1.x. x+5.x.x. x.x.")
│ > play()
│                                                                        │
│ Tracks                               Meters                            │
│ 1 Kick  delay off                    |████▆▃▁|                         │
│ 2 Snare delay 1/4 fb0.35             |▁▃▆█▇▅▂|                         │
│ 3 Hat   delay off                    |▁▁▂▂▃▃▅|                         │
│ Playhead ▶ step 06/16                                                    │
└────────────────────────────────────────────────────────────────────────┘
```

## REPL and object model

You work in a language-like REPL. Everything is an object with methods and properties. State is live and hot-swappable while audio runs.

### Core objects

* `song()`: holds global tempo, steps, swing, and an ordered list of tracks
* `track(name?)`: creates a track object. Returned object has chainable methods.
* `pattern(str|fn)`: sets a pattern on the current track. Accepts a visual string or a coded generator function.
* `sample(path)`: loads a sample for the track
* `quantise(grid)`: per-track quantisation, e.g. `1/16`, `1/8T` (stub in v0.2)
* `fx`: namespace for per-track effects like `.delay(...)`, `.reverb(...)`
* `remove()`, `mute(bool)`, `solo(bool)`, `gain(db)`

### Pattern types

* **Visual patterns**: strings like `"x..x x..x x..x x..x"`

  * Per-note pitch: `x+3`, `x-5`
  * Per-note velocity: `x@96` with 0–127 scale, or shorthand `x!` `x!!` `x!!!` mapped to 40, 80, 110
  * Probability: `x%35`
* **Coded patterns**: functions that yield steps with fields `{hit, pitch, vel, prob}`

  * Example generators: Euclidean, fills, random walks, Markov

### Coded pattern examples

```
# Define a 16-step Euclidean pattern with velocity accents
fn euclid(k, n) {
  let out = [];
  let pats = distribute(k, n);           # builtin helper
  for i in 0..n {
    if pats[i] { out.push({hit: true, pitch: 0, vel: i % 4 == 0 ? 105 : 85}); }
    else { out.push({hit: false}); }
  }
  out
}
let hh = track("Hat").sample("hat.wav");
hh.pattern(euclid(11, 16));
```

### Global functions

* `play()`, `stop()`, `bpm(n)`, `steps(n)`, `swing(percent)`
* `save("song.yaml")`, `open("song.yaml")`
* `list()`: prints tracks and key properties
* `meter(trackId?)`: shows peak and RMS for one or all tracks

### Syntax style

* Chainable calls: `track("Hat").sample("hat.wav").pattern("x.x.")`
* Let-binding: `let hh = track("Hat")`
* Properties readable via dot: `hh.name`, `hh.fx.delay.time`
* Inline help: `:help track`, `:doc pattern`
* Autocomplete for methods and file paths (sample selection completion; see `documentation/features/sample-autocomplete.md`)

### Minimal grammar

* Identifiers: `let name = ...`
* Calls: `name.method(args...)`
* Numbers: ints and floats, percentages as 0..1
* Strings: double quotes
* Comments: `# ...` end of line

## Live updates and visibility

Edits in the REPL apply to the live object graph while audio runs. The UI reflects changes instantly.

* Instant apply: `bpm`, `steps`, `swing`, `mute/solo/gain`, and FX params update without restarting transport.
* Safe boundaries: Pattern and timing changes take effect on the next step (or bar) boundary to avoid glitches.
* TUI visibility: Track list shows concise FX summaries; meters and playhead update continuously.
* Autocomplete: Tab-complete method names and sample paths rooted at `samples/`.

## Audio behaviour

* Sample rate default 48 kHz, 24‑bit internal float
* Each note within a pattern can apply its own pitch transpose in semitones using simple resampling
* Transpose changes duration with pitch in v0.1 (keeps engine simple)
* Tempo‑synced delay per track using circular buffer
* Mixer sums tracks to stereo out with headroom and soft clipper

## Architecture

**Modules**

1. REPL interpreter: tokenise, parse, evaluate to object graph
2. Object model: `Song`, `Track`, `Fx` with observable properties
3. UI TUI: REPL console, inspector panel, meters, playhead
4. Transport: BPM clock, swing, playhead
5. Pattern parser: text to events including per‑note pitch
6. Scheduler: event queue to audio thread with pre‑roll
7. Audio engine: sample playback, resampler, per‑track delay
8. Persistence: serialise object graph to YAML song file

**Data flow**
REPL edits object graph → diff emits changes → scheduler updates triggers → audio engine renders.

**Threading**

* Main thread: REPL, TUI, object changes
* Audio thread: callback render, lock‑free mailbox for change events

## Technical stack recommendation

Rust from day one.

### Core crates

* **Audio engine and timing**: Kira for precise clocks, tracks and effects routing. CPAL under the hood for I/O. citeturn0search0turn0search21turn0search2
* **Decoding**: Symphonia for WAV, AIFF, FLAC, MP3, OGG, MP4. citeturn0search1turn0search9
* **Resampling**: start with linear or cubic. Later use a crate like rubato for higher quality. citeturn0search17
* **TUI**: Ratatui for the UI panels. citeturn0search3turn0search11
* **REPL**: rustyline for input, history and completion. citeturn0search12turn0search4
* **Scripting for coded patterns**: Rhai embedded language, documented and sandboxed. citeturn0search5turn0search19
* **Parsing helpers**: chumsky or nom for the pattern string parser. citeturn0search6turn0search14
* **CLI**: clap for flags and subcommands.

### Why this set

Reliable cross‑platform audio and decoding, a proven TUI, and a small embedded language so users can write pattern code without recompiling.

### Extensibility plan

* **Effect trait**: `Effect::process(&mut frame, &mut state)` with metadata and params
* **Pattern provider trait**: `PatternGen::next(step) -> StepState`
* **Registry**: map of named effects and generators, discoverable in the REPL via `:list effects` and `:list gens`
* **Plugin hooks**: load out‑of‑tree effects and pattern gens via dynamic linking in a later version
* **Scripting**: expose safe wrappers in Rhai so community scripts can add generators without Rust

## File formats

Primary interface is the REPL script history. Persistence uses YAML. Later we can add a `.groove` script export of the session.

```yaml
# song.yaml
bpm: 120
steps: 16
swing: 0
repeat: true

tracks:
  - name: "Kick"
    sample: "samples/909/kick.wav"
    delay:
      on: false
      time: "1/4"
      feedback: 0.35
      mix: 0.25
    pattern: { Visual: "x... x... x... x..." }

  - name: "Snare"
    sample: "samples/909/snare.wav"
    delay:
      on: true
      time: "1/4"
      feedback: 0.35
      mix: 0.25
    pattern: { Visual: ". . . .  x . . .  . . . .  x . . ." }

  - name: "Hat"
    sample: "samples/909/hat.wav"
    delay:
      on: false
      time: "1/4"
      feedback: 0.35
      mix: 0.25
    pattern: { Visual: "x+2.x.x. x-1.x. x+5.x.x. x.x." }
```

## Visualisation

* REPL console with syntax highlight and suggestions
* Inspector panel showing selected object properties live
* Grid with moving playhead
* Per‑track meters using unicode blocks
* Waveform preview on sample load
* Colours per track and for warnings

## Project layout

```
.
├── documents/
│   └── features/
│       └── 0001-cli-groovebox-spec.md
├── samples/
│   └── 909/...
├── src/
│   ├── app.py
│   ├── ui/
│   │   ├── tui.py
│   │   └── widgets.py
│   ├── audio/
│   │   ├── engine.py
│   │   ├── sample.py
│   │   ├── delay.py
│   │   └── resample.py
│   ├── pattern/
│   │   ├── parser.py
│   │   └── scheduler.py
│   └── io/
│       ├── song.py
│       └── paths.py
└── tests/
```

## MVP v0.1 scope

* Rust binary with Ratatui UI, rustyline REPL and Kira engine
* 3 tracks, 1 bar of 16 steps, 60 to 180 BPM
* Load WAV and AIFF via Symphonia, fall back to WAV if issues
* Per-note pitch with simple resample, per-note velocity 0–127
* Per-track delay with time division, feedback, mix
* Visual patterns and coded patterns in Rhai
* Save and load to YAML, including coded pattern references

## Non goals for v0.1

* MIDI in or out
* Sync to external clock
* Sample slicing
* Independent time‑stretch with stable quality
* Recording

## Testing strategy

All development follows strict TDD. No code is written without a failing test first. No exceptions.

### Rules

* Write the minimal failing test before any new logic
* Commit test and implementation together once green
* Keep coverage above 90% for core modules (pattern, scheduler, audio engine)
* Unit, integration and property tests for every module
* Golden tests for pattern parsing including velocity and pitch
* Property tests for scheduler alignment at bar boundaries
* Audio render checksum tests for short patterns
* Visual unit tests using Ratatui snapshot harness
* CI rejects merges that fail tests or drop coverage
* Review and Update documents after every change to contract, addition of feature, new object or methods

## Roadmap

* v0.1 prototype as above
* v0.2 probability and ties, basic filter, quantisation
* v0.3 multi‑bar patterns, per‑track swing, tempo changes, export to WAV
* v1 Rust engine, stable low latency, richer effects, headless mode

## Reference documents

- REPL commands: `documentation/features/repl-commands.md`
  (YAML is the persistence format; see examples above)
- Sample autocomplete: `documentation/features/sample-autocomplete.md`
- TUI design: `documentation/features/tui-design.md`
