# APP MODULE KNOWLEDGE

## OVERVIEW
`src/app` owns runtime orchestration: state machine, key -> `Action` mapping, and async task result transitions.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Event loop | `src/app/mod.rs` | key poll, draw, task drain, spinner tick |
| Action execution | `src/app/mod.rs` | `execute_action` is mutation boundary |
| Async callbacks | `src/app/mod.rs` | `TaskResult` handling and status updates |
| State definitions | `src/app/state.rs` | `AppState`, `ListState`, `ChangelogState` |
| Keymaps by state | `src/app/handler.rs` | `handle_key` dispatch by `StateKind` |
| Changelog lock flow | `src/app/handler.rs`, `src/app/mod.rs` | confirm dialog -> lock URL -> spawn lock |

## CONVENTIONS
- Keep key handlers mostly pure: return `Action`, do not spawn tasks from handler.
- Perform side effects in `App::execute_action` and task-spawn helpers only.
- Preserve the two-stage changelog flow: `LoadingChangelog(parent)` before `Changelog(Box<...>)`.
- When refreshing list data, clamp cursor and clear stale selections/status maps.
- Status text lifecycle is centralized in `App` (`StatusMessage` expiry + render handoff).

## STATE INVARIANTS
- `ListState.table_state` selection tracks `cursor`; update both on cursor moves.
- `ListState.busy` gates mutating commands (`u`, `U`, `r`, `c`) to prevent overlap.
- `ChangelogState.confirm_lock` is modal state; `Some(idx)` only if commit list non-empty.
- On changelog close/failure, restore parent list state instead of rebuilding ad hoc.

## ANTI-PATTERNS
- Do not mutate `AppState` directly inside `handler.rs` beyond local cursor/selection changes.
- Do not bypass `Action` and call services straight from input handling.
- Do not add new background result variants without handling them in `handle_task_result`.
- Do not forget to clear `UpdateStatus::Updating` markers on both success and error paths.
