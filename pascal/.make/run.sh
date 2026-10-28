#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BIN_PATH="$ROOT_DIR/bin/pforth"

if [ ! -f "$BIN_PATH" ]; then
    "$ROOT_DIR/.make/build.sh"
fi

if [ -f "$BIN_PATH" ]; then
    exec "$BIN_PATH" "$@"
else
    echo "Error: Binary not found at $BIN_PATH"
    exit 1
fi
