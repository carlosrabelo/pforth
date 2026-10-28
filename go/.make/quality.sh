#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "Running go fmt check..."
diffs=$(gofmt -l .)
if [ -n "$diffs" ]; then
    echo "Unformatted files found:"
    echo "$diffs"
    exit 1
fi

echo "Running go vet..."
go vet ./...

echo "Go quality checks completed."
