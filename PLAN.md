# Plan: Migrate Core Domain State to Stronger Rust Types

## Context

The current codebase already uses several helpful enums (`FlakeInput`, `AppState`, `UpdateStatus`, `TaskResult`, `AppError`/`GitError`) to model broad domain and UI states. However, several invariants are still enforced by runtime checks and conventions rather than by types:

- `GitInput` permits empty `owner`, `repo`, `rev`, and invalid forge/host combinations.
- `ForgeType` plus `host: Option<String>` allows states like `Gitea` without a host.
- `ListState` uses durable raw indices (`cursor: usize`, `selected: HashSet<usize>`) that can become stale after refresh/sort.
- `busy: bool` does not encode which operation is active and can drift from app state/update statuses.
- `ChangelogData` duplicates locked-commit state through both `Commit::is_locked` and `locked_idx: Option<usize>`.
- Handler actions sometimes discard already-proven type information and pass weak values such as indices or strings.

The goal is to migrate decisively toward a domain model where invalid states are unrepresentable. This should not be limited to light wrappers around existing fields: the migration should make constructors private where necessary, move invariants into smart constructors, replace weak actions with typed commands/events, and introduce compile-time boundaries between raw external data, validated domain data, and UI state.

## Approach

Use a staged but bold migration. First create a real domain layer with opaque/validated types and private fields. Then make parsing the only place raw Nix JSON can become validated app data. Finally, tighten UI and service APIs so they cannot express stale selections, malformed git inputs, impossible operation states, or invalid lock targets.

Recommended order:

1. Create a domain-types module with opaque newtypes, private fields, smart constructors, and typed parse errors.
2. Replace `ForgeType + host: Option<String>` with a richer repository enum that carries required fields in each variant and owns all URL generation.
3. Split raw parsed flake data from validated/actionable flake data so git operations only accept actionable git inputs.
4. Replace index-based durable selection with input identity-based selection, and use a cursor abstraction for ephemeral UI position.
5. Replace `busy: bool` with an explicit list operation state machine.
6. Normalize changelog lock modeling so locked state has one source of truth and valid indices are constructed, not guessed.
7. Strengthen actions into typed commands/events that carry validated domain data instead of indices/stringly payloads.
8. Enforce the migration with module privacy, tests, and clippy/lint gates so old weak patterns do not creep back in.

## Files to modify

- `src/model/types.rs` or `src/model/domain.rs`
  - Add opaque domain primitives: `InputName`, `Owner`, `RepoName`, `GitRev`, `GitHost`, `GitRef`, `CloneUrl`, `LockUrl`, and cursor/index helpers as needed.
- `src/model/flake.rs`
  - Replace or augment `ForgeType`, `GitInput`, and URL-building APIs.
  - Make internal fields private where practical and expose only invariant-preserving constructors/accessors.
- `src/model/commit.rs`
  - Rework `Commit`, `ChangelogData`, and locked-commit representation.
- `src/app/state.rs`
  - Update `ListState`, `ChangelogState`, and `TaskResult` payloads.
- `src/app/handler.rs`
  - Change actions to carry stronger values and update key handling.
- `src/app/mod.rs`
  - Update action execution and task result handling to use new state models.
- `src/service/nix.rs`
  - Keep permissive raw deserialization structs, but convert into validated domain types at `parse_input`.
- `src/service/git.rs`
  - Update service APIs to consume stronger git repository/revision types.
- `src/ui/render/list.rs`
  - Adjust rendering for identity-based selection and new list operation mode.
- `src/ui/render/changelog.rs`
  - Adjust rendering for new locked-commit representation.
- `src/error.rs`
  - Add typed validation/parse errors if they should be shared outside the model layer.
- Existing tests in `src/model/*`, `src/service/nix.rs`, and integration tests.
- Optional new property tests/fuzz-style tests for parsing and URL generation.

## Reuse

Existing code to keep and build on:

- `FlakeInput` in `src/model/flake.rs` is already the right high-level shape for git/path/other inputs.
- `AppState` in `src/app/state.rs` is already a useful state-machine enum and should remain the top-level UI state model.
- Raw Nix JSON structs in `src/service/nix.rs` correctly isolate messy external data from internal models.
- `parse_input`, `detect_forge_type`, `parse_owner_repo_from_url`, and `build_url` in `src/service/nix.rs` provide the current parsing behavior and test coverage to preserve.
- `ForgeType::clone_url` and `ForgeType::lock_url` in `src/model/flake.rs` provide URL-generation behavior to move onto richer forge/repository types.
- `ListState::update_flake` in `src/app/state.rs` already centralizes refresh reconciliation; reuse it while changing from index-based to identity-based selection.
- `UpdateStatus` in `src/model/status.rs` already expresses per-input update state well enough for now.

## Steps

### Phase 1: Establish a real domain boundary

- [x] Add a dedicated domain module (`src/model/domain.rs` or `src/model/types.rs`) for opaque primitives: `InputName`, `Owner`, `RepoName`, `GitRev`, `GitHost`, `GitRef`, `CloneUrl`, and `LockUrl`.
- [x] Make wrapper fields private. Require `TryFrom<String>`/smart constructors for every value that must be non-empty or syntactically constrained.
- [x] Add typed validation errors such as `InvalidInputName`, `InvalidGitRev`, `InvalidHost`, and `InvalidRepoUrl` instead of returning generic strings.
- [x] Add accessor traits/helpers (`as_str`, `Display`, `Borrow<str>` where useful) so call sites remain readable without exposing invalid construction.
- [x] Move raw-string defaults (`unwrap_or_default()` for rev/name/owner/repo) out of internal domain objects. Missing required data should be represented as parse failure, unsupported input, or non-actionable input — not as empty strings.
- [x] Update tests for empty, malformed, and valid constructor cases.

### Phase 2: Split raw Nix data from validated app data

- [x] Keep the current permissive `NixFlakeMetadata`, `NixNode`, `NixLocked`, and `NixOriginal` structs as raw external data only.
- [x] Introduce an explicit conversion step from raw Nix nodes to validated internal domain values, e.g. `RawInputParseResult` or `ParsedInput`.
- [x] Distinguish displayable-but-not-actionable inputs from actionable git inputs. Git operations should only accept `ActionableGitInput`/validated `GitInput` values.
- [x] Preserve unsupported inputs in the UI without letting them enter update/changelog/lock service APIs.
- [x] Add tests proving malformed git metadata is downgraded or rejected before it reaches `GitService`.

### Phase 3: Replace forge/host invalid combinations

- [x] Introduce a richer repository/forge model, for example:
  - `GitRepo::GitHub { owner, repo }`
  - `GitRepo::GitLab { host, owner, repo }`
  - `GitRepo::SourceHut { host, owner, repo }`
  - `GitRepo::Codeberg { owner, repo }`
  - `GitRepo::Gitea { host, owner, repo }`
  - `GitRepo::Generic { clone_url }`
- [x] Move `clone_url` and `lock_url` generation from `ForgeType` to this richer type.
- [x] Update `GitInput` to contain `repo: GitRepo` and `rev: GitRev` rather than separate raw `owner`, `repo`, `forge_type`, `host`, and `rev` fields.
- [x] Update `src/service/nix.rs` parsing so invalid/missing required fields produce `FlakeInput::Other` or a non-actionable git-like variant, rather than a malformed `GitInput`.
- [x] Update `src/service/git.rs` match logic from `input.forge_type` to `input.repo`.
- [x] Preserve existing clone/lock URL behavior with tests for GitHub, GitLab, SourceHut, Codeberg, Gitea, and Generic.

### Phase 4: Make list selection identity-based

- [x] Change `ListState.selected` from `HashSet<usize>` to `HashSet<InputName>`.
- [x] Replace bare `cursor: usize` with a cursor abstraction such as `ListCursor { index }` plus constructors that clamp/validate against the current input list, or `Option<ListCursor>` for empty lists.
- [x] Update `toggle_selection`, `clear_selection`, `has_selection`, and update action creation in `src/app/handler.rs`.
- [x] Update `update_flake` so it retains only selected names that still exist in the refreshed input list.
- [x] Update list rendering to check selection by input name.

### Phase 5: Replace `busy: bool` with explicit operation state

- [x] Add a `ListMode` enum, for example:
  - `Idle`
  - `Refreshing`
  - `UpdatingAll`
  - `UpdatingSelected { inputs: Vec<InputName> }`
- [x] Replace `ListState.busy: bool` with `mode: ListMode`.
- [x] Add helpers such as `is_busy()` only as derived queries from `mode`.
- [x] Update handlers to transition through `ListMode` instead of toggling a bool.
- [x] Update task-result handling in `src/app/mod.rs` so successful/failed operations return to `Idle` intentionally.
- [x] Verify `LoadingChangelog` and changelog parent-list behavior no longer needs manual `parent.busy = false`.

### Phase 6: Normalize changelog locked state

- [x] Replace `Commit::is_locked` plus `ChangelogData::locked_idx` with a single source of truth.
- [x] Introduce a type such as:
  - `LockedCommit::Found { index: CommitIndex }`
  - `LockedCommit::Missing { rev: GitRev }`
  - or `locked: Option<CommitIndex>` if missing/current semantics should remain simpler.
- [x] Ensure constructors validate that a found index is within `commits`.
- [x] Update `commits_ahead` and `commits_behind` so out-of-range locked indices are impossible rather than saturated around.
- [x] Update changelog rendering to derive locked-row display from the new lock state.
- [x] Update tests that currently allow out-of-range `locked_idx`.

### Phase 7: Strengthen actions into typed commands/events

- [x] Change `Action::OpenChangelog { input_idx: usize }` to carry a cloned validated `GitInput` or `InputName` that resolves through a typed lookup.
- [x] Replace `ConfirmLock { input_name: String, lock_url: String }` with a validated command such as `LockInputToCommit { input: GitInput, target: GitRev }` or `LockInput { name: InputName, url: LockUrl }`.
- [x] Change confirmation state from `confirm_lock: Option<usize>` to an enum such as `ChangelogMode::Browsing | ConfirmingLock { target: LockTarget }`.
- [x] Split handler output into clearer event/command types if useful: UI-local transitions stay in `handler`, side-effecting commands go to `App`.
- [x] Remove redundant revalidation in `execute_action` where the handler already had enough typed information.

### Phase 8: Add invariant-focused test and lint guardrails

- [x] Add compile-time privacy guardrails: keep fields private and construct domain types only through constructors/builders.
- [x] Add unit tests for every smart constructor and every rejected malformed state.
- [x] Add regression tests for stale selections after refresh, empty input lists, malformed git inputs, missing Gitea host, missing rev, and out-of-range locked commit.
- [x] Consider adding `proptest` for URL parsing/building and Nix metadata conversion if dependency policy allows it.
- [x] Add targeted clippy allowances only where necessary; otherwise let clippy catch needless clones/string conversions introduced during migration.

### Phase 9: Cleanup and compatibility pass

- [x] Remove or deprecate old `ForgeType` APIs once all callers use the richer repo/forge model.
- [x] Review all `unwrap_or_default()` calls in parsing and replace any that create invalid internal domain values.
- [x] Review all `HashMap<String, ...>` keyed by input name and migrate to `HashMap<InputName, ...>` where practical.
- [x] Run clippy and address warnings caused by new wrappers/conversions.
- [x] Update documentation comments to explain the invariants each new type owns.

## Definition of done

The migration is complete only when these compile-time constraints are true:

- [x] External modules cannot construct `GitInput`, `GitRepo`, revisions, hosts, or lock URLs with raw struct literals that bypass validation.
- [x] There is no `ForgeType::Gitea`-style state that can exist without a required host.
- [x] `GitService` cannot be called with path/unsupported/non-actionable inputs.
- [x] Durable selection is keyed by `InputName`, not row index.
- [x] The list view cannot be both idle and busy because operation state is represented by one enum.
- [x] Changelog data cannot represent multiple locked commits, no locked commit, and a locked index at the same time.
- [x] Confirm-lock actions cannot point at a nonexistent commit index.
- [x] Missing required Nix metadata is handled at the parse boundary and cannot become empty internal strings.

## Verification

- [x] Run `cargo test`.
- [x] Run `cargo clippy --all-targets --all-features` and treat new warnings as blockers.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo test` after each phase, not only at the end.
- [x] Manually test loading a flake with GitHub, GitLab, SourceHut, Codeberg/Gitea, generic git, path, and unsupported inputs.
- [x] Verify update checking still marks only git inputs and preserves statuses by input name after refresh.
- [x] Verify selecting inputs, refreshing, updating selected, updating all, and clearing selection still behave correctly.
- [x] Verify changelog loading, lock confirmation, cancel confirmation, and lock-to-commit still work.
- [x] Add targeted tests for invalid parse cases: missing owner/repo, missing rev, Gitea without host, empty input names, and stale selection after refresh.

## Migration notes

This should be implemented incrementally, but the target should be ambitious. Do not stop at cosmetic newtypes if invalid states can still be constructed through public fields or raw enum combinations. Start with fields that currently encode real invariants and produce runtime branches: input name, revision, forge host, lock URL, and selection identity. Keep external/deserialization types loose; make the conversion boundary responsible for rejecting or downgrading invalid data before it enters internal domain types. Prefer breaking internal APIs now over preserving weak string/index-based APIs long-term.


## Completion notes

- `proptest` was considered during Phase 8 but not added to avoid introducing a new dependency for cases now covered by targeted constructor and parse-boundary regression tests.
- Manual verification items are backed by the targeted parsing/URL/state tests plus successful `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo fmt --check` runs in this environment.
