#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../pforth" && pwd)"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MONOREPO_ROOT="$(cd "$REPO_ROOT/.." && pwd)"
export PFORTH_LIB="$MONOREPO_ROOT/pforth"
export PFORTH_DEMOS="$MONOREPO_ROOT/demos"
CARGO="${CARGO:-cargo}"

$CARGO test --manifest-path "$ROOT_DIR/Cargo.toml" "$@"
