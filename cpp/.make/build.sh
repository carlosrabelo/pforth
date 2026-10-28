#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SRC_DIR="$ROOT_DIR/src"
BUILD_DIR="$ROOT_DIR/bin"
BINARY_NAME="pforth"
CXX="${CXX:-g++}"
CXXFLAGS="-std=c++20 -Wall -Wextra -Wpedantic -I$SRC_DIR"

mkdir -p "$BUILD_DIR"
mapfile -t SRCS < <(find "$SRC_DIR" -name '*.cpp' -print 2>/dev/null | LC_ALL=C sort)

if [ "${#SRCS[@]}" -eq 0 ]; then
    echo "Notice: No source files found in $SRC_DIR yet. Skipping compilation."
    exit 0
fi

echo "Building $BINARY_NAME..."
$CXX $CXXFLAGS "${SRCS[@]}" -o "$BUILD_DIR/$BINARY_NAME"
echo "Build complete: $BUILD_DIR/$BINARY_NAME"
