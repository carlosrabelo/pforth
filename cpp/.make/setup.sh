#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LIB_DIR="$ROOT_DIR/lib"

mkdir -p "$LIB_DIR"
if [ ! -f "$LIB_DIR/catch.hpp" ]; then
    echo "Downloading Catch2..."
    curl -sSL https://raw.githubusercontent.com/catchorg/Catch2/v2.x/single_include/catch2/catch.hpp -o "$LIB_DIR/catch.hpp"
fi
echo "Setup complete."
