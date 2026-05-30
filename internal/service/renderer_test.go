package service

import (
	"strings"
	"testing"
)

func TestRenderSafeHTMLRemovesUnsafeMarkup(t *testing.T) {
	t.Parallel()

	renderer := NewRendererService()
	html, excerpt, err := renderer.RenderSafeHTML("# Hi\n<script>alert(1)</script>\n[link](javascript:alert(1))")
	if err != nil {
		t.Fatalf("render markdown: %v", err)
	}
	if strings.Contains(strings.ToLower(html), "<script") {
		t.Fatalf("unsafe script tag should be removed: %s", html)
	}
	if strings.Contains(strings.ToLower(html), "javascript:") {
		t.Fatalf("unsafe javascript URL should be removed: %s", html)
	}
	if !strings.Contains(html, "<h1>Hi</h1>") {
		t.Fatalf("expected heading HTML: %s", html)
	}
	if !strings.Contains(excerpt, "Hi") {
		t.Fatalf("expected excerpt to contain text: %s", excerpt)
	}
}
