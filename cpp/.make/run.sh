#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../pforth" && pwd)"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MONOREPO_ROOT="$(cd "$REPO_ROOT/.." && pwd)"
export PFORTH_LIB="${PFORTH_LIB:-$MONOREPO_ROOT/pforth}"
BIN_PATH="$REPO_ROOT/bin/pforth"

if [ ! -f "$BIN_PATH" ]; then
    "$(dirname "$0")/build.sh"
fi

if [ -f "$BIN_PATH" ]; then
    exec "$BIN_PATH" "$@"
else
    echo "Error: Binary not found at $BIN_PATH"
    exit 1
fi
