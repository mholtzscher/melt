# AGENTS.md - AI Agent Guidelines for melt-rs

A terminal UI for managing Nix flake inputs, built with Rust and ratatui.

**Stack**: Rust 2021 (MSRV 1.75), tokio, ratatui, crossterm, git2, reqwest, serde, clap, thiserror

## Commands

All commands should be run through Nix tooling. **Never commit code unless explicitly prompted by the user.**

```bash
# Build
nix develop -c cargo build                    # dev build
nix develop -c cargo build --release          # release (LTO + strip)

# Test
nix develop -c cargo test                     # all tests
nix develop -c cargo test test_name           # single test
nix develop -c cargo test module_name::       # module tests
nix develop -c cargo test -- --nocapture      # with output

# Lint
nix develop -c cargo fmt                      # format code
nix develop -c cargo fmt -- --check           # check only
nix develop -c cargo clippy                   # run lints
nix develop -c cargo clippy --fix             # auto-fix

# Build and run with Nix
nix build                      # build package
nix run                        # run directly
```

## Project Structure

```
src/
  main.rs          # CLI entry, clap args
  lib.rs           # Public API, re-exports
  error.rs         # thiserror error types
  config.rs        # Config structs with Default
  event.rs         # Input handling
  tui.rs           # Terminal RAII wrapper
  app/             # Core: state.rs (state machine), handler.rs (actions)
  model/           # Domain: flake.rs, commit.rs, status.rs
  service/         # External: nix.rs, git.rs (APIs + git2 fallback)
  ui/              # Rendering: theme.rs (Catppuccin), render/
tests/             # Integration tests, fixtures in test-data/
```

## Code Style

### Imports
Order: std -> external crates (alphabetically) -> crate internal, separated by blank lines.

### Types
```rust
#[derive(Debug, Clone)]           // minimum for most types
#[derive(Debug, Clone, Copy)]     // small types without heap
#[derive(Debug, Clone, PartialEq, Eq)]  // when equality needed

#[derive(Debug, Clone, Default)]
pub enum UpdateStatus {
    #[default]
    Unknown,
    // ...
}
```

### Error Handling
```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("No flake.nix found in {0}")]
    FlakeNotFound(PathBuf),
    #[error("Git error: {0}")]
    Git(#[from] GitError),
}
pub type AppResult<T> = Result<T, AppError>;
```

### Module Organization
Keep `mod.rs` minimal - just declarations and re-exports:
```rust
mod commit;
mod flake;
pub use commit::{ChangelogData, Commit};
pub use flake::{FlakeData, FlakeInput};
```

### Async Patterns
- `tokio::spawn_blocking` for CPU-bound/blocking ops (git2)
- `tokio::time::timeout` for timeouts
- `CancellationToken` for cooperative cancellation
- `Arc<Semaphore>` to limit concurrency

### Serde
```rust
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]  // for unused JSON fields
struct NixLocked {
    #[serde(rename = "type", default)]
    type_: Option<String>,
}
```

### State Machine Pattern
State is an enum; handlers return Action enums instead of mutating directly:
```rust
pub enum AppState { Loading, Error(String), List(ListState), ... }
pub enum Action { None, Quit, UpdateSelected(Vec<String>), ... }
```

### Naming
- Types: `PascalCase` (FlakeInput, GitService)
- Functions: `snake_case` (check_updates, cursor_down)
- Constants: `SCREAMING_SNAKE_CASE` (BG_HIGHLIGHT)
- Acronyms as words: `Url` not `URL`

### Documentation
- `//!` for module docs, `///` for items
- `#[cfg(test)]` for test modules in same file

### UI/Theme
- Colors in `ui/theme.rs` (Catppuccin Mocha)
- Use semantic names: `SUCCESS`, `ERROR`, `TEXT_DIM`
- No hardcoded colors in render functions

### Testing
- Unit tests in `#[cfg(test)]` module in same file
- Integration tests in `tests/`
- Fixtures in `test-data/`
- Use `tempfile` for filesystem tests

## Key Files Reference

| File | Purpose |
|------|---------|
| `src/app/state.rs` | AppState enum, ListState, ChangelogState |
| `src/app/handler.rs` | Key handlers, Action enum |
| `src/service/git.rs` | API calls + git2 fallback for forges |
| `src/service/nix.rs` | `nix flake metadata/update` commands |
| `src/model/flake.rs` | FlakeInput, GitInput, ForgeType |
| `src/ui/theme.rs` | Color constants (Catppuccin Mocha) |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GITHUB_TOKEN` / `GH_TOKEN` | GitHub API auth (increases rate limit) |
| `RUST_BACKTRACE` | Set to `1` for backtraces (set in dev shell) |
