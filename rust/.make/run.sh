#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BIN_PATH="$ROOT_DIR/bin/pforth"

if [ ! -f "$BIN_PATH" ]; then
    "$ROOT_DIR/.make/build.sh"
fi

exec "$BIN_PATH" "$@"
