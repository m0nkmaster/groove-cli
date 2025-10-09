# REPL Sample Autocomplete

This document specifies the behavior, UX, and implementation details for autocompleting sample paths when choosing a sample for a track in the REPL.

## Goals

- Provide fast, accurate autocompletion of sample paths rooted at the `samples/` folder.
- Make picking samples fluid while live-coding; suggestions appear as you type.
- Minimize latency and avoid blocking the audio/transport thread.

## Scope

- Autocomplete for the `sample <track_idx> "path"` command argument.
- Default root is the workspace `samples/` folder, but support absolute and relative paths.
- File types: `wav`, `aiff`, `aif`, `flac`, `mp3`, `ogg`, `m4a` (configurable).

## UX Requirements

- Trigger: When the cursor is inside the quoted path after `sample <idx>`, pressing Tab shows suggestions.
- Display: A dropdown list with up to 20 candidates (scrollable if supported by the line editor), ordered by:
  1. Prefix match > substring match
  2. Direct children before deeper descendants
  3. Alphabetical tie-breaker (case-insensitive)
- Directories: show with trailing `/` and allow drilling down.
- Relative base:
  - No prefix or starts with `samples/` → search under `samples/`.
  - Starts with `./` or `../` → resolve relative to current working directory.
  - Absolute path (`/` or drive prefix on Windows) → complete from that root.
- Hidden files and directories (dot-prefixed) are excluded by default.
- Case-insensitive matching on macOS/Windows; case-sensitive on Linux.
- If there is a single unambiguous completion, Tab completes inline; otherwise, it prints the common prefix and shows the list.

## Performance Requirements

- Directory scanning must not block the audio thread. All FS work occurs on the main/REPL thread.
- Cache directory listings in-memory with a TTL (default 2 seconds) to reduce FS churn while typing.
- Keep the cache small (LRU by directory path, max 256 entries) to keep memory bounded.

## Edge Cases

- Large sample libraries: Suggestion rendering truncates to 20 visible items with paging.
- Missing `samples/` folder: Show a hint “Create samples/ to enable library completion.” Still allow absolute/relative path completion.
- Symlinks: Resolve symlinks for display but return the user-typed path.
- Spaces in paths: always surround with quotes; completion inserts escaped quotes if needed inside the string.
- Non-audio files: filtered out unless the extension whitelist is disabled via config.

## Configuration

- `repl.autocomplete.sample_root` (string, default: `"samples"`)
- `repl.autocomplete.max_results` (int, default: 20)
- `repl.autocomplete.extensions` (array, default: audio set above)
- `repl.autocomplete.cache_ttl_ms` (int, default: 2000)

Configuration source order: CLI flag > env var > project config file > defaults.

## REPL Grammar Impact

- The `sample` command retains its syntax: `sample <idx> "path"`.
- Completion is context-aware: only the last token (inside quotes) is considered for path completion.

## Implementation Sketch (Rust)

- Integrate with `rustyline` via a custom `Completer` that inspects the current line buffer.
- Detect the `sample` command and identify the quoted path span.
- Resolve a base directory from the partially typed path.
- Query a cache (keyed by directory path) for directory entries; refresh on miss/TTL expiry.
- Filter by prefix/substring; sort by rules above; format candidates with `/` for directories.
- Return `rustyline::CompletionType::List` with replacements anchored to the span.

Threading: all completion logic runs on the REPL thread; audio engine is independent.

## Testing Strategy

- Unit tests for the path resolver (base dir, typed prefix → candidate list).
- Unit tests for filtering (prefix vs substring) and sorting.
- Tests for extension filtering, hidden file exclusion, case sensitivity per platform.
- Cache behavior tests (expiry, LRU eviction).
- Snapshot tests for completion list formatting (golden outputs).

## Future Enhancements

- Preview small waveform or duration metadata in suggestion list (optional).
- Fuzzy matching (e.g., `fzf`-style scoring) for large libraries.
- Configurable sample roots (multiple folders).
- Async filesystem watcher to push updates into the cache.

