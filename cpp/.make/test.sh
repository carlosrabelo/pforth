#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../pforth" && pwd)"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MONOREPO_ROOT="$(cd "$REPO_ROOT/.." && pwd)"
export PFORTH_LIB="$MONOREPO_ROOT/pforth"
export PFORTH_DEMOS="$MONOREPO_ROOT/demos"
TESTS_DIR="$ROOT_DIR/tests"
SRC_DIR="$ROOT_DIR/src"
LIB_DIR="$ROOT_DIR/lib"
BUILD_DIR="$REPO_ROOT/bin"
TEST_BINARY="pforth-tests"
CXX="${CXX:-g++}"
CXXFLAGS="-std=c++20 -Wall -Wextra -Wpedantic -I$SRC_DIR -I$LIB_DIR"

mkdir -p "$BUILD_DIR"
mapfile -t TEST_SRCS < <(find "$TESTS_DIR" -name '*.cpp' -print 2>/dev/null | LC_ALL=C sort)

if [ "${#TEST_SRCS[@]}" -eq 0 ]; then
    echo "Notice: No test files found in $TESTS_DIR yet. Skipping tests."
    exit 0
fi

mapfile -t LIB_SRCS < <(find "$SRC_DIR" -name '*.cpp' ! -name 'main.cpp' -print 2>/dev/null | LC_ALL=C sort)

echo "Compiling test runner..."
$CXX $CXXFLAGS "${TEST_SRCS[@]}" "${LIB_SRCS[@]}" -o "$BUILD_DIR/$TEST_BINARY"
echo "Running tests..."
cd "$REPO_ROOT"
"$BUILD_DIR/$TEST_BINARY"
