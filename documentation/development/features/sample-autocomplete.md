# Sample Completion (Implemented)

This document describes how sample completion works today in both the classic REPL and the TUI.

## Source of truth

- REPL helper + completer: `src/repl/completer.rs`
- Sample resolution + suggestions (for errors): `src/repl/mod.rs` (`resolve_sample_path`, `find_similar_samples`)

## Supported audio extensions

Completion scans `samples/` recursively and includes:

- `wav`, `mp3`, `ogg`, `flac`, `aiff`, `aif`

## Primary UX: `track ~ …`

The recommended user-facing form is:

```text
kick ~ 909/kick<Tab>
```

Behavior:

- Completion is **context-aware** when a `~` is present in the input line.
- Matching is **token-based**: the query is split on whitespace and `/`, and a candidate must contain **all tokens** (any order).
- Suggestions are displayed as a shortened `dir/file` form, but insertion uses the full path.

Multi-word queries work without quoting:

```text
kick ~ linn snare class<Tab>
```

Bracketed queries are also supported (useful when you want to keep chaining more segments after the sample):

```text
kick ~[linn snare class]<Tab>
```

## TUI cycling

In the TUI, when multiple matches exist, Tab **cycles** through candidates and inserts the currently selected one into the command line.

## Secondary UX: `sample …` / `preview …`

Completion also supports the index-based commands:

```text
sample 1 "samples/…"<Tab>
preview "samples/…"<Tab>
```

Quoted and unquoted paths are supported, but quoting is recommended for paths with spaces.

## TUI vs REPL differences

- **REPL** (`--repl`): uses a cached `GrooveHelper` instance, so completion is fast after startup.
- **TUI** (default): calls `complete_for_tui`, which constructs a helper on demand.

If completion ever becomes slow for large libraries, the first place to optimize is avoiding repeated full-directory scans in the TUI completion path.



