package mcp

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"blogWeb/internal/model"
	"blogWeb/internal/testutil"
)

func TestIssueTokenAndAuthenticateHTTP(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "test-client", []string{ScopeBlogRead}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	if token == "" {
		t.Fatalf("expected token")
	}

	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(`{"jsonrpc":"2.0","id":1}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	req.Header.Set("Authorization", "Bearer "+token)

	client, authErr := server.authenticateHTTP(context.Background(), req, ScopeBlogRead)
	if authErr != nil {
		t.Fatalf("authenticate http: %+v", authErr)
	}
	if client == nil || client.Name != "test-client" {
		t.Fatalf("unexpected client: %+v", client)
	}
	if client.TokenHash == token {
		t.Fatalf("token should not be stored in plaintext")
	}
}

func TestAuthenticateHTTPRejectsBadOriginAndScope(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "test-client", []string{ScopeBlogRead}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}

	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(`{"jsonrpc":"2.0","id":1}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", "https://evil.example")
	req.Header.Set("Authorization", "Bearer "+token)
	if _, authErr := server.authenticateHTTP(context.Background(), req, ScopeBlogRead); authErr == nil || authErr.Code != "invalid_origin" {
		t.Fatalf("expected invalid origin reject, got %+v", authErr)
	}

	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	if _, authErr := server.authenticateHTTP(context.Background(), req, ScopePublish); authErr == nil || authErr.Code != "forbidden_scope" {
		t.Fatalf("expected forbidden scope reject, got %+v", authErr)
	}
}

func TestRevokeTokenDisablesAuthentication(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "revoked-client", []string{ScopeBlogRead}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	if err := server.RevokeToken(context.Background(), "revoked-client"); err != nil {
		t.Fatalf("revoke token: %v", err)
	}

	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(`{"jsonrpc":"2.0","id":1}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	req.Header.Set("Authorization", "Bearer "+token)

	if _, authErr := server.authenticateHTTP(context.Background(), req, ScopeBlogRead); authErr == nil || authErr.Code != "invalid_token" {
		t.Fatalf("expected revoked token reject, got %+v", authErr)
	}
}

func TestWriteAuditStoresDigestWithoutRawPayload(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	payload := `{"content":"# secret body","content_base64":"aGVsbG8="}`
	server.writeAudit(context.Background(), nil, "http", "tool_call", "tools/call", ScopeDraftWrite, "success", "req-1", "127.0.0.1", "", payload)

	var log model.MCPAuditLog
	if err := app.DB.WithContext(context.Background()).First(&log).Error; err != nil {
		t.Fatalf("query audit log: %v", err)
	}
	if log.PayloadDigest == "" {
		t.Fatalf("expected payload digest")
	}
	if strings.Contains(log.PayloadDigest, "secret body") || strings.Contains(log.PayloadDigest, "aGVsbG8=") {
		t.Fatalf("payload digest should not contain raw payload: %s", log.PayloadDigest)
	}
}
