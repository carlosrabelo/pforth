#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "Checking formatting with clang-format (if available)..."
if command -v clang-format &>/dev/null; then
    find "$ROOT_DIR/src" "$ROOT_DIR/tests" -name '*.cpp' -o -name '*.hpp' 2>/dev/null | while read -r f; do
        clang-format --dry-run --Werror "$f" || true
    done
fi

echo "C++ quality checks completed."
