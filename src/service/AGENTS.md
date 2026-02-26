# SERVICE MODULE KNOWLEDGE

## OVERVIEW
`src/service` is external-boundary code: Nix CLI calls plus forge update/changelog retrieval with API-first and git2 fallback logic.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Service exports | `src/service/mod.rs` | crate-visible service surface |
| Nix metadata load | `src/service/nix.rs` | `load_metadata` + JSON parse pipeline |
| Nix update/lock | `src/service/nix.rs` | `update_inputs`, `update_all`, `lock_input` |
| Nix process safety | `src/service/nix.rs` | timeout + cancellation + stderr mapping |
| Update checks | `src/service/git.rs` | concurrent fan-out with semaphore |
| API-first per forge | `src/service/git.rs` | GitHub/GitLab compare endpoints |
| git2 fallback | `src/service/git.rs` | bare clone cache, fetch, revwalk logic |
| Changelog load | `src/service/git.rs` | API commits first, fallback to local history |

## CONVENTIONS
- Treat service methods as IO boundaries: inputs validated at edge, typed models returned.
- Guard all long-running external calls with timeout + cancellation checks.
- For git2 work, use `tokio::task::spawn_blocking`; keep async executor non-blocking.
- Keep forge specialization narrow: API path for known hosts, generic path via git2.
- Keep clone URL generation centralized (`ensure_clone_url`, forge helpers).

## FAILURE MODEL
- Nix command failures map to `AppError::NixCommandFailed` with trimmed stderr.
- Network/API parse issues map to `GitError::NetworkError`; non-success API status can fallback to git2.
- Cancellation is cooperative; return early on cancelled token, abort outstanding join-set work.
- Rate-limit behavior is explicit for GitHub (403/429 + remaining header check).

## ANTI-PATTERNS
- Do not shell out to `git` CLI in service paths; keep git ops in `git2` implementation.
- Do not run blocking git2 operations directly in async context.
- Do not duplicate Nix metadata parsing structs outside `nix.rs` without hard reason.
- Do not skip semaphore limits when adding new parallel git checks.
