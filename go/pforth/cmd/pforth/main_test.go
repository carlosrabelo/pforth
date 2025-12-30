package main

import (
	"bytes"
	"strings"
	"testing"
)

func TestParseArgs(t *testing.T) {
	// No arguments
	cfg := ParseArgs([]string{"pforth"})
	if len(cfg.FilePaths) != 0 {
		t.Fatalf("expected 0 files, got %d", len(cfg.FilePaths))
	}

	// Single file
	cfg = ParseArgs([]string{"pforth", "script.fs"})
	if len(cfg.FilePaths) != 1 || cfg.FilePaths[0] != "script.fs" {
		t.Fatalf("unexpected files: %v", cfg.FilePaths)
	}

	// Multiple files
	cfg = ParseArgs([]string{"pforth", "file1.fs", "file2.fs"})
	if len(cfg.FilePaths) != 2 || cfg.FilePaths[0] != "file1.fs" || cfg.FilePaths[1] != "file2.fs" {
		t.Fatalf("unexpected files: %v", cfg.FilePaths)
	}
}

func TestRunMissingFile(t *testing.T) {
	cfg := Config{FilePaths: []string{"nonexistent_file_xyz_123.fs"}}
	in := &bytes.Buffer{}
	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}

	code := Run(cfg, in, out, errOut)
	if code != 1 {
		t.Fatalf("expected exit code 1, got %d", code)
	}

	errStr := errOut.String()
	if !strings.Contains(errStr, "CANNOT OPEN: nonexistent_file_xyz_123.fs") {
		t.Fatalf("expected 'CANNOT OPEN' error message, got: %q", errStr)
	}
}

func TestRunInteractiveEOF(t *testing.T) {
	cfg := Config{}
	in := &bytes.Buffer{}
	out := &bytes.Buffer{}
	errOut := &bytes.Buffer{}

	code := Run(cfg, in, out, errOut)
	if code != 0 {
		t.Fatalf("expected exit code 0, got %d", code)
	}
}
