# Tick: instructions for agents

Tick is a native GNOME client for Todoist in Rust + GTK4 + libadwaita.

## Read first

- `SPEC.md`: confirmed requirements. Do not change scope without the owner.
- `DESIGN.md`: binding visual specification. UI quality is the top priority.
- `docs/architecture.md`: crates, data flow, environment variables.
- `docs/testing.md`: test layers and the verified E2E harness facts.
- `docs/api/`: Todoist API v1 OpenAPI spec (`openapi.json`) and doc pages (`pages/*.md`).
  Use these as the source of truth for API behavior, not memory.

## Rules

- Rust 2024 edition. `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass.
- `tick-core` must not depend on GTK.
- All tests use `cargo test`. No Python.
- Every new user-visible behavior gets an E2E test that drives the real app.
- Every interactive control has an accessible name.
- User-visible strings go through gettext. Chinese copy uses ASCII punctuation with English
  spacing (`3 个任务`, `今天, 明天.`).
- Commits follow Conventional Commits (`type(scope): subject`). Never use `--no-verify`.
- Never commit secrets or tokens.

## Commands

- `just check`: fmt, clippy, unit and integration tests.
- `just e2e`: E2E tests (needs mutter, at-spi2-core, dbus).
- `just screenshots`: write design-review PNGs to `target/screenshots/`.
  Narrow screenshots use a 600x800 virtual monitor.
- `just run`: run the app against the real API (token from keyring).
- `just run-fake`: run the app against a seeded fake server.
