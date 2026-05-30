package service

import (
	"strings"
	"testing"
	"time"
)

func TestSlugifyAndValidate(t *testing.T) {
	t.Parallel()

	got := Slugify(" Hello, Blog / World 2026 ")
	if got != "hello-blog-world-2026" {
		t.Fatalf("unexpected slug: %s", got)
	}
	if !IsValidSlug(got) {
		t.Fatalf("slug should be valid")
	}
	if IsValidSlug("Bad Slug") {
		t.Fatalf("invalid slug unexpectedly accepted")
	}
}

func TestBuildExcerptAndCoverImagePath(t *testing.T) {
	t.Parallel()

	excerpt := BuildExcerpt("# Title\nhello **world**", 20)
	if !strings.Contains(excerpt, "Title hello world") {
		t.Fatalf("unexpected excerpt: %s", excerpt)
	}

	validCases := []string{
		"",
		"/uploads/2026/05/abc.jpg",
		"/uploads/2026/12/aBc_123.webp",
		"https://example.com/cover.jpg",
	}
	for _, value := range validCases {
		if !ValidateCoverImagePath(value) {
			t.Fatalf("expected valid cover image path: %s", value)
		}
	}

	invalidCases := []string{
		"http://evil.example/x.jpg",
		"https://",
		"/uploads/../../x.jpg",
		"/uploads/2026/5/x.jpg",
		"/uploads/2026/05/x.svg",
	}
	for _, value := range invalidCases {
		if ValidateCoverImagePath(value) {
			t.Fatalf("expected invalid cover image path: %s", value)
		}
	}
}

func TestEncodeDecodeCursor(t *testing.T) {
	t.Parallel()

	cursorText, err := EncodeCursor(Cursor{
		IsPinned:    1,
		PublishedAt: time.Date(2026, 5, 14, 8, 0, 0, 0, time.UTC),
		ID:          7,
	})
	if err != nil {
		t.Fatalf("encode cursor: %v", err)
	}

	cursor, err := DecodeCursor(cursorText)
	if err != nil {
		t.Fatalf("decode cursor: %v", err)
	}
	if cursor.ID != 7 || cursor.IsPinned != 1 {
		t.Fatalf("unexpected cursor: %+v", cursor)
	}

	if _, err := DecodeCursor("{bad json"); err == nil {
		t.Fatalf("expected decode failure")
	}
}
