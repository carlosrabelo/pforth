#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
if [ ! -f "$ROOT_DIR/bin/pforth" ]; then
    "$ROOT_DIR/.make/build.sh"
fi
exec "$ROOT_DIR/bin/pforth" "$@"
