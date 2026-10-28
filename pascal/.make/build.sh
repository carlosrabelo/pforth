#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SRC_DIR="$ROOT_DIR/src"
BUILD_DIR="$ROOT_DIR/bin"
OBJ_DIR="$ROOT_DIR/obj"
BINARY_NAME="pforth"
FPC="${FPC:-fpc}"
FPCFLAGS="-Mobjfpc -Sh -gl -O2 -FE$BUILD_DIR -FU$OBJ_DIR -Fu$SRC_DIR"

mkdir -p "$BUILD_DIR" "$OBJ_DIR"

if [ ! -f "$SRC_DIR/main.pas" ]; then
    echo "Notice: No main.pas found in $SRC_DIR yet. Skipping compilation."
    exit 0
fi

echo "Building $BINARY_NAME..."
$FPC $FPCFLAGS "$SRC_DIR/main.pas" -o"$BINARY_NAME"
echo "Build complete: $BUILD_DIR/$BINARY_NAME"
