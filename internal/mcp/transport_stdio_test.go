package mcp

import (
	"bytes"
	"context"
	"encoding/json"
	"strings"
	"testing"

	"blogWeb/internal/testutil"
)

type stdioResponse struct {
	Result json.RawMessage `json:"result"`
	Error  *struct {
		Code    int            `json:"code"`
		Message string         `json:"message"`
		Data    map[string]any `json:"data"`
	} `json:"error"`
}

func TestStdioDisablesWriteCapabilitiesByDefault(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	app.Config.MCP.StdioWriteEnabled = false
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)

	var output bytes.Buffer
	err := server.ServeStdio(context.Background(), strings.NewReader("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}\n"), &output, &bytes.Buffer{})
	if err != nil {
		t.Fatalf("serve stdio tools/list: %v", err)
	}

	var response stdioResponse
	if err := json.Unmarshal(output.Bytes(), &response); err != nil {
		t.Fatalf("decode tools/list response: %v", err)
	}
	if strings.Contains(output.String(), "create_article_draft") || strings.Contains(output.String(), "publish_article") {
		t.Fatalf("write tools should not be exposed when stdio writes disabled: %s", output.String())
	}

	output.Reset()
	err = server.ServeStdio(context.Background(), strings.NewReader("{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"create_article_draft\",\"arguments\":{\"title\":\"x\",\"content\":\"# body\"}}}\n"), &output, &bytes.Buffer{})
	if err != nil {
		t.Fatalf("serve stdio write tool: %v", err)
	}
	if err := json.Unmarshal(output.Bytes(), &response); err != nil {
		t.Fatalf("decode write reject response: %v", err)
	}
	if response.Error == nil || response.Error.Code != 403 {
		t.Fatalf("expected 403 stdio write reject, got %s", output.String())
	}
}

func TestStdioAcceptsLargePreviewRequests(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	app.Config.MCP.StdioWriteEnabled = true
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)

	largeMarkdown := strings.Repeat("a", 80*1024)
	request := map[string]any{
		"jsonrpc": "2.0",
		"id":      1,
		"method":  "tools/call",
		"params": map[string]any{
			"name": "preview_markdown",
			"arguments": map[string]any{
				"content": largeMarkdown,
			},
		},
	}
	data, err := json.Marshal(request)
	if err != nil {
		t.Fatalf("marshal request: %v", err)
	}

	var output bytes.Buffer
	if err := server.ServeStdio(context.Background(), strings.NewReader(string(data)+"\n"), &output, &bytes.Buffer{}); err != nil {
		t.Fatalf("serve stdio large request: %v", err)
	}

	var response stdioResponse
	if err := json.Unmarshal(output.Bytes(), &response); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if response.Error != nil {
		t.Fatalf("expected large stdio preview request to succeed, got %s", output.String())
	}
}
