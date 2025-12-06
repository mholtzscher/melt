# melt

A TUI for managing Nix flake inputs.

## Features

- View all flake inputs with revision, type, and last modified time
- Check for available updates (shows commit count behind)
- Update individual or all inputs
- View changelog for GitHub inputs
- Lock inputs to specific commits

## Installation

### With Nix (recommended)

```bash
nix run github:your-username/melt
```

Or add to your flake:

```nix
{
  inputs.melt.url = "github:your-username/melt";
}
```

### From source

Requires [Bun](https://bun.sh):

```bash
git clone https://github.com/your-username/melt
cd melt
bun install
bun run start
```

## Usage

```bash
# Run in current directory
melt

# Run on a specific flake
melt /path/to/flake
```

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate up/down |
| `space` | Select input |
| `u` | Update selected inputs |
| `U` | Update all inputs |
| `c` | View changelog (GitHub only) |
| `r` | Refresh |
| `esc` | Back / Clear selection / Quit |

### Changelog view

| Key | Action |
|-----|--------|
| `j/k` | Navigate commits |
| `enter` | Lock to selected commit |
| `esc` | Back to list |

## GitHub API

Set `GITHUB_TOKEN` for higher rate limits when checking updates:

```bash
export GITHUB_TOKEN=ghp_...
melt
```

Also supports `GH_TOKEN` and `GITHUB_PAT`.

## License

MIT
