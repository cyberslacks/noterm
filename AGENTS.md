# Repository Guidelines

## Project Structure & Module Organization

Noterm is a Rust 2021 terminal-native Markdown notes application. `src/main.rs`
owns the Tokio event loop and application startup. Keep feature logic in its
domain module: `app.rs` for shared state and events, `tui/` for rendering,
layouts, key handling, and widgets, `notes/` for Markdown/frontmatter and file
scanning, `search/` for Tantivy and vector search, and `llm/` for providers.
Use `db/`, `git/`, `import/`, `kazam/`, `export/`, and `tasks/` for their
respective integrations. Release automation is in `.github/workflows/`.

## Build, Test, and Development Commands

```bash
cargo build                 # compile a debug binary
cargo run                   # launch the TUI locally
cargo test                  # run all unit tests
cargo test freshness        # run matching tests only
cargo fmt                   # apply Rust formatting
cargo clippy                # check common Rust issues
cargo build --release       # build optimized distribution binary
```

For interactive feature checks, follow `TESTING.md`; it describes a sample
notes directory and expected TUI behavior. The default notes directory is
`~/notes`, while user configuration is `~/.config/noterm/config.toml`.

## Coding Style & Naming Conventions

Use standard `rustfmt` output (four-space indentation) and run `cargo fmt`
before submitting changes. Follow Rust naming: `snake_case` for functions,
modules, and variables; `PascalCase` for types and enum variants; and
`SCREAMING_SNAKE_CASE` for constants. Keep UI work split between
`tui/widgets/<feature>.rs`, `renderer.rs`, and `keys.rs`.

The TUI must not write to stdout after initialization, because it corrupts the
screen. Use `tracing` for diagnostics. Run blocking filesystem, Git, SQLite,
or search work through `tokio::task::spawn_blocking` and return results as
`AppEvent`s.

## Testing Guidelines

Place focused unit tests beside the code in a `#[cfg(test)] mod tests` block;
existing examples are in `src/notes/freshness.rs` and
`src/notes/annotations.rs`. Name tests for observable behavior, such as
`parses_weekly_review_interval`. Run `cargo test` and `cargo clippy` for every
change; manually test affected key flows using `TESTING.md` when changing the
TUI.

## Commit & Pull Request Guidelines

Recent history uses concise, imperative subjects (for example, `Add README`)
and versioned release summaries (`v0.3.0: Kazam integration...`). Keep commits
single-purpose. Pull requests should explain user-visible behavior, identify
configuration or migration effects, link relevant issues, and include terminal
screenshots or recordings for visual TUI changes. Confirm formatting, tests,
and linting in the PR description.
