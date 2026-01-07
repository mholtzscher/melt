# melt

A terminal UI for managing Nix flake inputs. View, update, and lock your flake inputs interactively.

This is a Rust rewrite of [melt](https://github.com/anomalyco/melt), built with [ratatui](https://ratatui.rs).

## Features

- **View flake inputs** - See all inputs with name, type, revision, and last modified time
- **Check for updates** - Background checks show how many commits each input is behind
- **Update inputs** - Update selected inputs or all at once
- **View changelog** - Browse commit history for any git input
- **Lock to commit** - Select a specific commit to lock an input to
- **Multi-forge support** - GitHub, GitLab, SourceHut, Codeberg, and generic git

## Installation

### With Nix

```bash
nix run github:anomalyco/melt-rs
```

Or add to your flake inputs:

```nix
{
  inputs.melt.url = "github:anomalyco/melt-rs";
}
```

### From source

```bash
git clone https://github.com/anomalyco/melt-rs
cd melt-rs
nix develop
cargo build --release
```

## Usage

```bash
# Run in current directory
melt

# Run in specific flake directory
melt /path/to/flake
```

## Key Bindings

### List View

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` | Toggle selection |
| `u` | Update selected inputs |
| `U` | Update all inputs |
| `c` | View changelog for current input |
| `r` | Refresh flake metadata |
| `q` / `Esc` | Quit |

### Changelog View

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` | Select commit for locking |
| `y` | Confirm lock to selected commit |
| `n` | Cancel lock |
| `q` / `Esc` | Back to list |

## Status Column

The STATUS column shows update status for git inputs:

- ` ` (empty) - Not yet checked
- `...` - Currently checking
- `ok` - Up to date
- `+N` - N commits behind (e.g., `+5` means 5 commits behind)

## Architecture

```
src/
├── main.rs           # CLI entry point
├── app.rs            # Application state machine
├── tui.rs            # Terminal setup/teardown
├── event.rs          # Input handling
├── error.rs          # Error types
├── model/            # Data structures
│   ├── flake.rs      # FlakeData, FlakeInput, ForgeType
│   ├── commit.rs     # Commit, ChangelogData
│   └── status.rs     # UpdateStatus
├── service/          # Business logic
│   ├── nix.rs        # Nix flake commands
│   └── git.rs        # Git operations (via git2)
├── ui/               # Rendering
│   └── theme.rs      # Catppuccin Mocha colors
└── util/
    └── time.rs       # Relative time formatting
```

## Development

```bash
# Enter dev shell
nix develop

# Run
cargo run -- /path/to/flake

# Test
cargo test

# Build release
cargo build --release
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GITHUB_TOKEN` | GitHub personal access token for API authentication |
| `GH_TOKEN` | Alternative to `GITHUB_TOKEN` (used by `gh` CLI) |

Setting a GitHub token increases the API rate limit from 60 to 5000 requests/hour.

## Requirements

- Nix with flakes enabled
- Git (for changelog features, via libgit2)
- SSH agent (for private repos)

## License

MIT
