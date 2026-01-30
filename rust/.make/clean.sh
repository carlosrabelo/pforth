#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../pforth" && pwd)"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CARGO="${CARGO:-cargo}"

$CARGO clean --manifest-path "$ROOT_DIR/Cargo.toml"
rm -f "$REPO_ROOT/bin/pforth"
