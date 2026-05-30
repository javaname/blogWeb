package compat

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"testing"
	"time"

	"blogWeb/internal/handler"
	"blogWeb/internal/mcp"
	"blogWeb/internal/service"
	"blogWeb/internal/testutil"
)

type goldenHTTPResponse struct {
	Status  int                 `json:"status"`
	Headers map[string][]string `json:"headers,omitempty"`
	Cookies []goldenCookie      `json:"cookies,omitempty"`
	Body    any                 `json:"body,omitempty"`
}

type goldenCookie struct {
	Name     string `json:"name"`
	Value    string `json:"value"`
	Path     string `json:"path,omitempty"`
	MaxAge   int    `json:"max_age,omitempty"`
	HttpOnly bool   `json:"http_only,omitempty"`
	Secure   bool   `json:"secure,omitempty"`
	SameSite string `json:"same_site,omitempty"`
}

var tokenLikePattern = regexp.MustCompile(`^[A-Za-z0-9_-]{24,}$`)

var volatileTimeFields = map[string]bool{
	"created_at": true,
	"updated_at": true,
}

func TestGenerateGoldenBaseline(t *testing.T) {
	app := testutil.NewApp(t)
	category := testutil.SeedCategory(t, app, "Technology", "technology")
	publishedAt := time.Date(2026, 5, 29, 8, 0, 0, 0, time.UTC)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Rust Migration Baseline",
		Content:     "# Baseline\n\n<script>alert(1)</script>\n\nStable text.",
		CategoryID:  &category.ID,
		Status:      "published",
		PublishedAt: &publishedAt,
	})

	router := handler.NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()

	writeGolden(t, "http/healthz.json", performHTTP(router, http.MethodGet, "/healthz", "", nil))
	writeGolden(t, "http/public_articles.json", performHTTP(router, http.MethodGet, "/api/articles?limit=2", "", nil))
	writeGolden(t, "http/public_article_detail.json", performHTTP(router, http.MethodGet, "/api/articles/"+article.Slug, "", nil))
	writeGolden(t, "http/admin_csrf_unauthorized.json", performHTTP(router, http.MethodGet, "/api/admin/csrf-token", "", nil))

	login := performHTTP(router, http.MethodPost, "/api/admin/login", `{"username":"admin","password":"admin-password"}`, map[string]string{"Content-Type": "application/json"})
	writeGolden(t, "http/admin_login.json", login)

	mcpServer := mcp.NewServer(
		app.Config,
		slog.New(slog.NewTextHandler(io.Discard, nil)),
		app.DB,
		app.RateLimiter,
		app.Renderer,
		app.Categories,
		app.Articles,
		app.Uploads,
	)
	writeGolden(t, "mcp/http_missing_token.json", performMCPHTTP(mcpServer.HTTPHandler(), "", `{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}`))

	token, err := mcpServer.IssueToken(context.Background(), "golden-reader", []string{"blog.read", "blog.category.read"}, "http")
	if err != nil {
		t.Fatalf("issue mcp token: %v", err)
	}
	writeGolden(t, "mcp/http_initialize.json", performMCPHTTP(mcpServer.HTTPHandler(), token, `{"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}`))
}

func performHTTP(router http.Handler, method, target, body string, headers map[string]string) goldenHTTPResponse {
	req := httptest.NewRequest(method, target, strings.NewReader(body))
	for key, value := range headers {
		req.Header.Set(key, value)
	}
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	return normalizeResponse(resp.Result(), resp.Body.Bytes())
}

func performMCPHTTP(handler http.Handler, token, body string) goldenHTTPResponse {
	req := httptest.NewRequest(http.MethodPost, "/mcp", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", "2025-11-25")
	req.Header.Set("Origin", "https://chatgpt.com")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp := httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	return normalizeResponse(resp.Result(), resp.Body.Bytes())
}

func normalizeResponse(resp *http.Response, body []byte) goldenHTTPResponse {
	defer resp.Body.Close()
	result := goldenHTTPResponse{
		Status:  resp.StatusCode,
		Headers: normalizeHeaders(resp.Header),
		Cookies: normalizeCookies(resp.Cookies()),
	}
	if len(bytes.TrimSpace(body)) == 0 {
		return result
	}
	var payload any
	if err := json.Unmarshal(body, &payload); err == nil {
		result.Body = normalizeJSON(payload)
		return result
	}
	result.Body = strings.TrimSpace(string(body))
	return result
}

func normalizeHeaders(headers http.Header) map[string][]string {
	allow := map[string]bool{
		"Content-Type":            true,
		"Content-Security-Policy": true,
		"X-Content-Type-Options":  true,
		"Referrer-Policy":         true,
		"X-Frame-Options":         true,
		"Location":                true,
		"WWW-Authenticate":        true,
	}
	result := map[string][]string{}
	for key, values := range headers {
		canonical := http.CanonicalHeaderKey(key)
		if !allow[canonical] {
			continue
		}
		copied := append([]string(nil), values...)
		sort.Strings(copied)
		result[canonical] = copied
	}
	if len(result) == 0 {
		return nil
	}
	return result
}

func normalizeCookies(cookies []*http.Cookie) []goldenCookie {
	if len(cookies) == 0 {
		return nil
	}
	result := make([]goldenCookie, 0, len(cookies))
	for _, cookie := range cookies {
		value := cookie.Value
		if tokenLikePattern.MatchString(value) {
			value = "<TOKEN>"
		}
		result = append(result, goldenCookie{
			Name:     cookie.Name,
			Value:    value,
			Path:     cookie.Path,
			MaxAge:   cookie.MaxAge,
			HttpOnly: cookie.HttpOnly,
			Secure:   cookie.Secure,
			SameSite: sameSiteName(cookie.SameSite),
		})
	}
	sort.Slice(result, func(i, j int) bool { return result[i].Name < result[j].Name })
	return result
}

func sameSiteName(value http.SameSite) string {
	switch value {
	case http.SameSiteDefaultMode:
		return "Default"
	case http.SameSiteLaxMode:
		return "Lax"
	case http.SameSiteStrictMode:
		return "Strict"
	case http.SameSiteNoneMode:
		return "None"
	default:
		return ""
	}
}

func normalizeJSON(value any) any {
	switch typed := value.(type) {
	case map[string]any:
		result := make(map[string]any, len(typed))
		for key, item := range typed {
			switch key {
			case "csrf_token", "token":
				result[key] = "<TOKEN>"
			case "last_seen":
				result[key] = "<TIMESTAMP>"
			default:
				if volatileTimeFields[key] {
					result[key] = "<TIMESTAMP>"
					continue
				}
				result[key] = normalizeJSON(item)
			}
		}
		return result
	case []any:
		result := make([]any, len(typed))
		for index, item := range typed {
			result[index] = normalizeJSON(item)
		}
		return result
	case string:
		return typed
	default:
		return typed
	}
}

func writeGolden(t *testing.T, relativePath string, value any) {
	t.Helper()
	path := filepath.Join(projectRoot(t), "tests", "golden", relativePath)
	data, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		t.Fatalf("marshal golden %s: %v", relativePath, err)
	}
	data = append(data, '\n')
	if os.Getenv("UPDATE_GOLDEN") == "1" {
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatalf("create golden dir: %v", err)
		}
		if err := os.WriteFile(path, data, 0o644); err != nil {
			t.Fatalf("write golden %s: %v", relativePath, err)
		}
		return
	}
	existing, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read golden %s: %v; run with UPDATE_GOLDEN=1 to create it", relativePath, err)
	}
	if !bytes.Equal(existing, data) {
		t.Fatalf("golden mismatch %s\nwant sha256=%x\ngot  sha256=%x", relativePath, sha256.Sum256(existing), sha256.Sum256(data))
	}
}

func projectRoot(t *testing.T) string {
	t.Helper()
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("getwd: %v", err)
	}
	return filepath.Clean(filepath.Join(wd, "..", ".."))
}
