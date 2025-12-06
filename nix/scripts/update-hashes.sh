#!/usr/bin/env bash

set -euo pipefail

DUMMY="sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
HASH_FILE="$REPO_ROOT/nix/hashes.json"

cd "$REPO_ROOT"

if [ ! -f "$HASH_FILE" ]; then
  cat >"$HASH_FILE" <<EOF
{
  "nodeModules": "$DUMMY"
}
EOF
fi

cleanup() {
  rm -f "${BUILD_LOG:-}"
}

trap cleanup EXIT

write_hash() {
  local value="$1"
  local temp
  temp=$(mktemp)
  jq --arg value "$value" '.nodeModules = $value' "$HASH_FILE" >"$temp"
  mv "$temp" "$HASH_FILE"
}

echo "Setting dummy hash..."
write_hash "$DUMMY"

BUILD_LOG=$(mktemp)

echo "Building to discover correct hash..."
CORRECT_HASH=""

if nix build 2>"$BUILD_LOG"; then
  # Build succeeded - hash the output
  CORRECT_HASH=$(nix eval --raw '.#packages.'$(nix eval --impure --expr 'builtins.currentSystem')'.default.node_modules.outPath' 2>/dev/null | xargs nix hash path --sri 2>/dev/null || true)
fi

if [ -z "$CORRECT_HASH" ]; then
  # Extract from error message
  CORRECT_HASH="$(grep -E 'got:\s+sha256-[A-Za-z0-9+/=]+' "$BUILD_LOG" | awk '{print $2}' | head -n1 || true)"
fi

if [ -z "$CORRECT_HASH" ]; then
  echo "Failed to determine correct hash."
  echo "Build log:"
  cat "$BUILD_LOG"
  exit 1
fi

write_hash "$CORRECT_HASH"

echo "Hash updated: $CORRECT_HASH"
