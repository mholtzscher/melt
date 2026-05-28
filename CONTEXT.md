# Melt

Melt helps users inspect, update, and lock Nix flake inputs from an interactive terminal UI.

## Language

**Flake**:
A Nix project with declared **Flake Inputs** and lock data. In Melt, the **Flake** is the thing currently being inspected or updated.
_Avoid_: repository, project, workspace

**Flake Input**:
A dependency declared by a Nix flake and recorded in its lock data. A **Flake Input** may point to a git repository, local path, or another supported source.
_Avoid_: dependency, package, input

**Git Input**:
A **Flake Input** whose source is a git repository on a **Forge** or generic git host.
_Avoid_: git dependency, repository input

**Forge**:
A service that hosts git repositories used by **Git Inputs**, such as GitHub, GitLab, SourceHut, Codeberg, or Gitea.
_Avoid in product/domain language_: provider, platform. Technical field names like `host` are acceptable when referring to a network hostname.

**Path Input**:
A **Flake Input** whose source is a local filesystem path.
_Avoid_: local dependency, local input

**Unsupported Input**:
A **Flake Input** whose source Melt can display but cannot inspect for commits or update through **Commit History**. In code this is currently represented as `OtherInput`.
_Avoid in user-facing copy_: other input

**Locked Revision**:
The exact revision of a **Flake Input** currently recorded in the **Flake’s** lock data.
_Avoid_: current version, installed version, selected version

**Upstream Revision**:
The revision currently available from the source that a **Git Input** follows.
_Avoid_: latest version, remote revision, head

**Update**:
A change that moves one or more **Flake Inputs** to different **Locked Revisions**.
_Avoid_: upgrade, refresh

**Update Check**:
A non-mutating operation that compares a **Git Input’s** **Locked Revision** with its **Upstream Revision** and reports whether commits are available.
_Avoid_: update, refresh

**Lock to Commit**:
An **Update** that moves a **Git Input** to a specific commit chosen by the user.
_Avoid_: pin, checkout

**Refresh**:
Reload Melt’s view of the current **Flake** without intentionally changing **Locked Revisions**.
_Avoid_: update, reload

**Commit History**:
The commits Melt shows for a **Git Input** so the user can inspect changes and choose a commit for **Lock to Commit**. This is currently called `Changelog` in parts of the codebase.
_Avoid in new user-facing copy_: changelog, release notes, history

## Example dialogue

Developer: What does Melt show when I open a **Flake**?
Domain expert: Melt shows the **Flake Inputs** and each input’s **Locked Revision**.

Developer: When should a user choose **Update** instead of **Refresh**?
Domain expert: **Refresh** only reloads Melt’s view of the **Flake**. **Update** changes one or more **Locked Revisions**.

Developer: When does **Commit History** matter?
Domain expert: **Commit History** matters for a **Git Input** when the user wants to inspect commits and **Lock to Commit** instead of accepting the **Upstream Revision** chosen by a normal **Update**.
