# AGENTS.md

This file defines agent and developer instructions for this repository. Its scope applies to the entire repo unless another AGENTS.md exists deeper in a subdirectory that overrides specifics there.

## Primary Commands
- ALWAYS use TDD (Test-Driven Development) for changes.
- Use succinct Conventional Commits for commit messages.

## Development Workflow (TDD)
- Red → Green → Refactor on every change.
- Write a failing test first that narrowly asserts the intended behavior.
- Implement the minimal code to make the new test pass; avoid extra features.
- Refactor for clarity and maintainability with all tests staying green.
- Keep tests deterministic, isolated, and fast; prefer unit tests over integration.
- Avoid network, filesystem, and time flakiness in tests; mock or fake where feasible.
- When behavior is ambiguous, add an executable test to capture the decision.

## Commits (Conventional Commits)
- Format: `<type>(optional-scope): <succinct subject>`
- Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `revert`.
- Subject: imperative mood, <= 72 chars, no trailing period.
- One logical change per commit; keep diffs focused and small.
- Bodies are optional; if used, be brief and actionable.
- Note: Within Codex CLI sessions, only commit when explicitly requested by the user. If you do commit, follow these rules.

## Code Changes
- Make the smallest possible change to satisfy tests and requirements.
- Do not fix or reformat unrelated code.
- Preserve existing public APIs unless the task requires a change; if changed, document and test it.
- Follow the existing code style and naming conventions observed in the repo.
- Prefer clarity over cleverness; avoid one-letter variable names in new/changed code.

## Testing & Validation
- Add or update tests for every code change that affects behavior.
- Prefer unit tests; add integration tests only when necessary.
- Keep tests hermetic; mock network/IO. If real IO is unavoidable, isolate behind interfaces.
- Run and iterate locally; ensure the full suite passes before concluding the task.

## Documentation
- Update user and developer docs when behavior, flags, or interfaces change.
- Keep docs succinct and task-focused; include minimal reproduction/usage snippets.

## Using Codex CLI (Agents)
- Before tool calls, add a short preamble describing the next action.
- For multi-step tasks, maintain a lightweight plan with `update_plan`; keep exactly one step in progress.
- Edit files with `apply_patch`. Do not add license headers unless explicitly requested.
- When reading files, keep chunks <= 250 lines; prefer `rg`/`fd` for search and discovery.
- Avoid destructive shell commands; never delete or rewrite history unless explicitly asked.

## Security & Secrets
- Do not commit secrets, tokens, or credentials.
- Redact sensitive values from logs, examples, and tests.

## Performance & Reliability
- Choose simple solutions first; avoid premature optimization.
- Add guardrails for error handling where missing and covered by tests.
- Keep external dependencies minimal; justify additions in tests/docs when required.

## Default Assumptions
- If the repository lacks a test harness, create minimal tests under `tests/` aligned with the project’s language and existing patterns.
- If uncertainty arises, ask clarifying questions or encode assumptions as tests.

