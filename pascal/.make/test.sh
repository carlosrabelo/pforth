#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../pforth" && pwd)"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MONOREPO_ROOT="$(cd "$REPO_ROOT/.." && pwd)"
export PFORTH_LIB="$MONOREPO_ROOT/pforth"
export PFORTH_DEMOS="$MONOREPO_ROOT/demos"
TESTS_DIR="$ROOT_DIR/tests"
SRC_DIR="$ROOT_DIR/src"
BUILD_DIR="$REPO_ROOT/bin"
UNITS_DIR="$ROOT_DIR/units"
TEST_BINARY="pforth-tests"
FPC="${FPC:-fpc}"
FPCFLAGS="-Mobjfpc -Sh -gl -O2 -FE$BUILD_DIR -FU$UNITS_DIR -Fu$SRC_DIR -Fu$TESTS_DIR"

mkdir -p "$BUILD_DIR" "$UNITS_DIR"

if [ ! -f "$TESTS_DIR/test_runner.pas" ]; then
    echo "Notice: No test_runner.pas found in $TESTS_DIR yet. Skipping tests."
    exit 0
fi

echo "Compiling test runner..."
$FPC $FPCFLAGS "$TESTS_DIR/test_runner.pas" -o"$TEST_BINARY"
echo "Running tests..."
cd "$REPO_ROOT"
"$BUILD_DIR/$TEST_BINARY"
