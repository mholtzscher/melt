# PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-26T08:10:46-06:00
**Commit:** 59f274f
**Branch:** main

## OVERVIEW
Rust TUI for Nix flake input management. Core loop in `src/app`, external IO in `src/service`, rendering in `src/ui/render`, fixtures in `test-data`.

## STRUCTURE
```text
./
├── src/
│   ├── main.rs            # binary entry
│   ├── lib.rs             # public API + re-exports
│   ├── app/               # runtime/state machine + action execution
│   ├── service/           # nix CLI + forge API/git2 fallback integration
│   ├── model/             # domain objects + display helpers
│   ├── ui/render/         # list/changelog view rendering
│   └── util/              # shared time formatting
├── tests/                 # integration tests (parser mirror)
├── test-data/             # flake fixtures used by tests
└── .github/workflows/     # CI, release, dep update automation
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Runtime loop | `src/app/mod.rs` | event poll, render, task result handling |
| Key handling | `src/app/handler.rs` | state-aware key maps, returns `Action` |
| State invariants | `src/app/state.rs` | `AppState`, `ListState`, `ChangelogState` |
| Nix operations | `src/service/nix.rs` | `nix flake metadata/update`, timeout/cancel |
| Forge/changelog | `src/service/git.rs` | API first, git2 fallback, concurrency limits |
| Rendering | `src/ui/render/list.rs`, `src/ui/render/changelog.rs` | table layouts + status/help bars |
| Domain mapping | `src/model/flake.rs`, `src/model/status.rs` | input model + status presentation |
| Integration fixtures | `tests/integration_tests.rs`, `test-data/` | parser behavior + fixture coverage |

## CODE MAP
LSP unavailable in this environment. Use file-level map above and module re-exports in `src/lib.rs`.

## CONVENTIONS
- Nix-first tooling path: run Cargo via `nix develop` in local/CI flows.
- Binary + library both declare module trees (`src/main.rs`, `src/lib.rs`); keep both in sync when adding modules.
- App input flow: key event -> `Action` -> `App::execute_action`; avoid mutating state directly in key handlers.
- Async guardrails: `CancellationToken`, `tokio::time::timeout`, semaphore-limited fan-out in git checks.
- Theme discipline: colors live in `src/ui/theme.rs`; renderers use semantic constants only.

## ANTI-PATTERNS (THIS PROJECT)
- Never commit unless explicitly requested by user.
- Do not bypass Nix tooling for regular build/test/lint commands.
- Do not hardcode colors inside render functions.
- Do not block tokio threads with git2 work; use `spawn_blocking` paths in service layer.
- Avoid exposing parser internals only for tests; prefer crate API where possible.

## UNIQUE STYLES
- Two-stage state model for changelog: `LoadingChangelog(ListState)` then `Changelog(Box<...>)` with parent state restore.
- Multi-forge strategy: GitHub/GitLab HTTP compare APIs, fallback to bare-repo git2 for generic/sourcehut/codeberg/gitea.
- TUI status UX couples spinner frames + status messages in both list and help bars.
- Release flow is Node semantic-release, but product build/test is Nix+Cargo.

## COMMANDS
```bash
# Build
nix develop -c cargo build
nix develop -c cargo build --release

# Test
nix develop -c cargo test
nix develop -c cargo test test_name
nix develop -c cargo test module_name::

# Lint/format
nix develop -c cargo fmt -- --check
nix develop -c cargo clippy --all-targets -- -D warnings

# Nix package checks
nix build
nix flake check
```

## NOTES
- Release workflow uses `npm install` + `semantic-release`; keep `package-lock.json` healthy.
- Nix package sets `doCheck = false`; canonical test signal is CI `cargo test` job.
- `tests/integration_tests.rs` mirrors parser logic; drift risk exists when `src/service/nix.rs` parsing changes.
