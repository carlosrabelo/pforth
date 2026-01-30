#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../pforth" && pwd)"
CARGO="${CARGO:-cargo}"

echo "Checking formatting with rustfmt..."
$CARGO fmt --manifest-path "$ROOT_DIR/Cargo.toml" --all -- --check
echo "Linting with clippy..."
$CARGO clippy --manifest-path "$ROOT_DIR/Cargo.toml" --all-targets -- -D warnings
echo "Rust quality checks completed."
