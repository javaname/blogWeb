package mcp

import (
	"encoding/base64"
	"strings"
	"testing"
)

func TestSchemaValidationHelpers(t *testing.T) {
	t.Parallel()

	if err := validateSlug("hello-world-1"); err != nil {
		t.Fatalf("validate slug: %v", err)
	}
	if err := validateSlug("Hello World"); err == nil {
		t.Fatalf("expected invalid slug")
	}
	if err := validateTitle(strings.Repeat("a", 121)); err == nil {
		t.Fatalf("expected long title reject")
	}
	if err := validateMarkdown(""); err == nil {
		t.Fatalf("expected empty markdown reject")
	}
	if err := validateCoverImage("http://evil/x.png"); err == nil {
		t.Fatalf("expected external cover reject")
	}
}

func TestValidateBase64Size(t *testing.T) {
	t.Parallel()

	okPayload := base64.StdEncoding.EncodeToString([]byte("abc"))
	if err := validateBase64Size(okPayload, 10); err != nil {
		t.Fatalf("expected valid base64: %v", err)
	}
	if err := validateBase64Size("bad", 10); err == nil {
		t.Fatalf("expected invalid base64 reject")
	}
	bigPayload := base64.StdEncoding.EncodeToString([]byte(strings.Repeat("a", 12)))
	if err := validateBase64Size(bigPayload, 5); err == nil {
		t.Fatalf("expected size reject")
	}
}
