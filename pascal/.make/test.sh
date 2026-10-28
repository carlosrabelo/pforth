#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TESTS_DIR="$ROOT_DIR/tests"
SRC_DIR="$ROOT_DIR/src"
BUILD_DIR="$ROOT_DIR/bin"
OBJ_DIR="$ROOT_DIR/obj"
TEST_BINARY="test_runner"
FPC="${FPC:-fpc}"
FPCFLAGS="-Mobjfpc -Sh -gl -O2 -FE$BUILD_DIR -FU$OBJ_DIR -Fu$SRC_DIR -Fu$TESTS_DIR"

mkdir -p "$BUILD_DIR" "$OBJ_DIR"

if [ ! -f "$TESTS_DIR/test_runner.pas" ]; then
    echo "Notice: No test_runner.pas found in $TESTS_DIR yet. Skipping tests."
    exit 0
fi

echo "Compiling test runner..."
$FPC $FPCFLAGS "$TESTS_DIR/test_runner.pas" -o"$TEST_BINARY"
echo "Running tests..."
"$BUILD_DIR/$TEST_BINARY"
