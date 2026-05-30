package service

import (
	"bytes"
	"strings"

	"github.com/microcosm-cc/bluemonday"
	"github.com/yuin/goldmark"
	"github.com/yuin/goldmark/extension"
	"github.com/yuin/goldmark/renderer/html"
)

type RendererService struct {
	md  goldmark.Markdown
	pol *bluemonday.Policy
}

func NewRendererService() *RendererService {
	md := goldmark.New(
		goldmark.WithExtensions(
			extension.GFM,
			extension.Table,
			extension.Strikethrough,
		),
		goldmark.WithRendererOptions(
			html.WithHardWraps(),
			html.WithUnsafe(),
		),
	)

	policy := bluemonday.UGCPolicy()
	policy.RequireNoFollowOnLinks(false)
	policy.AllowAttrs("class").OnElements("code", "pre")

	return &RendererService{
		md:  md,
		pol: policy,
	}
}

func (r *RendererService) RenderSafeHTML(markdown string) (string, string, error) {
	var buf bytes.Buffer
	if err := r.md.Convert([]byte(markdown), &buf); err != nil {
		return "", "", err
	}
	html := r.pol.Sanitize(buf.String())
	excerpt := BuildExcerpt(strings.TrimSpace(markdown), 200)
	return html, excerpt, nil
}
