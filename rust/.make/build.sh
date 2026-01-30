#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../pforth" && pwd)"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$REPO_ROOT/bin"
BINARY_NAME="pforth"
CARGO="${CARGO:-cargo}"

echo "Building $BINARY_NAME..."
$CARGO build --manifest-path "$ROOT_DIR/Cargo.toml"
mkdir -p "$BIN_DIR"
cp "$ROOT_DIR/target/debug/$BINARY_NAME" "$BIN_DIR/$BINARY_NAME"
echo "Build complete: $BIN_DIR/$BINARY_NAME"
