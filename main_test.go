package main

import (
	"os"
	"testing"
)

func TestRunDoesNotPanicWithoutExplicitSubcommand(t *testing.T) {
	originalArgs := os.Args
	t.Cleanup(func() {
		os.Args = originalArgs
	})
	originalDir, err := os.Getwd()
	if err != nil {
		t.Fatalf("get working directory: %v", err)
	}
	tempDir := t.TempDir()
	t.Cleanup(func() {
		if err := os.Chdir(originalDir); err != nil {
			t.Fatalf("restore working directory: %v", err)
		}
	})
	if err := os.Chdir(tempDir); err != nil {
		t.Fatalf("change working directory: %v", err)
	}

	os.Args = []string{"blogWeb"}
	if err := run(); err == nil {
		t.Fatalf("expected run to fail without config or redis, but not panic")
	}
}
