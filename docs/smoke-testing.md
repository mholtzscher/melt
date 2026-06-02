# Smoke Testing

Use this checklist to validate TUI behavior quickly without repeating slow fetches, rebuilds, or hanging on flakes that have no committed lock file.

## Scope

Smoke tests should answer: does the app start, render locked flakes, navigate/select inputs, show commit history, and perform update flows without corrupting fixtures?

They are not a full network or Nix registry validation pass.

## Fast path

Run from the repository root inside the dev shell.

```bash
cargo test
cargo clippy -- -D warnings
cargo build
```

Build before launching in Zellij. Prefer `target/debug/melt` over `cargo run` in TUI panes so each launch does not spend time compiling or streaming build output.

Use command timeouts for smoke checks so failures are loud and bounded:

```bash
timeout 120s cargo test
timeout 120s cargo clippy -- -D warnings
timeout 60s cargo build
```

## Fixture tiers

### Deterministic locked fixtures

Use these for routine smoke testing:

- `test-data/minimal`
- `test-data/all-forges`
- `test-data/github-heavy`
- `test-data/kitchen-sink`

These have `flake.lock` files and should render without requiring Nix to resolve a fresh lock graph.

### Slow or network-dependent fixtures

Use these only when specifically validating lock generation or Nix fetch behavior:

- `test-data/dev-tools`
- `test-data/mixed-sources`
- `test-data/nixos-config`

These do not all have committed lock files. They may hang at `Loading flake...` or fail on transient Nix/GitHub/network fetch errors. Treat that as an environment/input-resolution issue unless the app crashes or fails to recover.

## Zellij launch pattern

Start one app pane:

```bash
zellij run --name melt-minimal --cwd "$PWD" -- \
  timeout --foreground 30s target/debug/melt test-data/minimal
```

Find the pane id:

```bash
zellij action list-panes -j -c -s -t \
  | jq -r '.[] | select(.title=="melt-minimal") | .id'
```

Capture the screen:

```bash
zellij action dump-screen -p terminal_<id> --full
```

Close the pane when done:

```bash
zellij action send-keys "q" -p terminal_<id>
zellij action send-keys "Ctrl c" -p terminal_<id> 2>/dev/null || true
zellij action close-pane -p terminal_<id> 2>/dev/null || true
```

## Manual checks

On `test-data/minimal`:

1. Confirm list renders with `nixpkgs`.
2. Press `Space`; confirm footer shows `1 selected` and row changes to `[x]`.
3. Press `c`; confirm commit history opens.
4. Press `j`, then `Space`; confirm lock dialog opens.
5. Press `n`; confirm dialog closes.
6. Press `q`; confirm return to list view.

For update flows, never mutate committed fixtures directly. Copy a fixture first:

```bash
tmp=$(mktemp -d /tmp/melt-minimal-update.XXXXXX)
cp -a test-data/minimal/. "$tmp/"
zellij run --name melt-update --cwd "$PWD" -- \
  timeout --foreground 60s target/debug/melt "$tmp"
```

Then validate:

1. Press `Space`, then `u`; confirm selected input updates and selection clears.
2. Press `U`; confirm update-all completes or remains safely no-op if already current.
3. Close the pane and remove the temp directory when finished.

## Batch render check

Use this only for non-mutating render smoke tests. It launches each locked fixture, waits briefly for a list or error, captures a summary, then closes the pane. Every wait has a timeout, and each app process is wrapped in `timeout --foreground` so stale panes cannot run forever.

```bash
fixtures=(
  test-data/minimal
  test-data/all-forges
  test-data/github-heavy
  test-data/kitchen-sink
)

APP_TIMEOUT=30
PANE_WAIT_ATTEMPTS=30      # 30 * 0.2s = 6s
RENDER_WAIT_ATTEMPTS=20    # 20 * 0.5s = 10s
failed=0

for f in "${fixtures[@]}"; do
  name="melt-$(basename "$f")"
  echo "=== $f ==="
  zellij run --name "$name" --cwd "$PWD" -- \
    timeout --foreground "${APP_TIMEOUT}s" target/debug/melt "$f" >/dev/null

  pane=""
  for _ in $(seq 1 "$PANE_WAIT_ATTEMPTS"); do
    pane=$(zellij action list-panes -j -c -s -t \
      | jq -r --arg name "$name" '.[] | select(.title==$name) | .id' \
      | tail -1)
    [ -n "$pane" ] && break
    sleep 0.2
  done

  if [ -z "$pane" ]; then
    echo "FAIL: zellij pane did not appear before timeout"
    failed=1
    echo
    continue
  fi

  for _ in $(seq 1 "$RENDER_WAIT_ATTEMPTS"); do
    out=$(zellij action dump-screen -p "terminal_$pane" --full 2>/dev/null || true)
    echo "$out" | grep -Eq 'NAME[[:space:]]+TYPE|Error|Failed' && break
    sleep 0.5
  done

  screen=$(zellij action dump-screen -p "terminal_$pane" --full 2>/dev/null || true)
  if ! echo "$screen" | grep -Eq 'NAME[[:space:]]+TYPE|Error|Failed'; then
    echo "FAIL: app did not render a list or error before timeout"
    failed=1
  fi

  echo "$screen" \
    | grep -E 'NAME[[:space:]]+TYPE|\[[ x]\]|Error|Failed|Loading flake|j/k nav' \
    | head -12 || true

  timeout 2s zellij action send-keys "q" -p "terminal_$pane" 2>/dev/null || true
  sleep 0.2
  timeout 2s zellij action send-keys "Ctrl c" -p "terminal_$pane" 2>/dev/null || true
  timeout 2s zellij action close-pane -p "terminal_$pane" 2>/dev/null || true
  echo
done

exit "$failed"
```

## Avoiding common slowdowns

- Do not run `cargo run` inside every Zellij pane; build once and run `target/debug/melt`.
- Prefer locked fixtures for normal smoke tests.
- Use temp copies for `u`, `U`, and lock-confirm flows.
- Put bounded waits around every fixture; do not wait indefinitely on `Loading flake...`.
- Wrap app launches with `timeout --foreground` and cleanup commands with short `timeout` calls.
- Capture Zellij screens with `dump-screen --full` instead of relying on visual inspection alone.
- Clean up panes after each run so stale app instances do not confuse later checks.
