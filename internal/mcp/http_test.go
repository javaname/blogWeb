package mcp

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strconv"
	"strings"
	"testing"
	"time"

	webhandler "blogWeb/internal/handler"
	"blogWeb/internal/model"
	"blogWeb/internal/service"
	"blogWeb/internal/testutil"
)

type rpcResponseEnvelope struct {
	Result json.RawMessage  `json:"result"`
	Error  *json.RawMessage `json:"error"`
}

func performMCPHTTPCall(t *testing.T, app *testutil.App, handler http.Handler, token string, payload string) *httptest.ResponseRecorder {
	t.Helper()

	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(payload))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp := httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	return resp
}

func TestMCPHTTPAuthorizationFailures(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	handler := server.HTTPHandler()

	resp := performMCPHTTPCall(t, app, handler, "", `{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}`)
	if resp.Code != http.StatusUnauthorized {
		t.Fatalf("expected auth required 401, got %d", resp.Code)
	}
	if !strings.Contains(resp.Header().Get("WWW-Authenticate"), "Bearer") {
		t.Fatalf("expected bearer challenge header, got %q", resp.Header().Get("WWW-Authenticate"))
	}

	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(`{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", "2099-01-01")
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	req.Header.Set("Authorization", "Bearer invalid")
	resp = httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	if resp.Code != http.StatusBadRequest {
		t.Fatalf("expected invalid protocol 400, got %d", resp.Code)
	}
}

func TestMCPHTTPForbiddenScopeAndInvalidOrigin(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "reader", []string{ScopeBlogRead}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	publishCall := `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"publish_article","arguments":{"id":1}}}`

	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(publishCall))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", "https://evil.example")
	req.Header.Set("Authorization", "Bearer "+token)
	resp := httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	if resp.Code != http.StatusForbidden {
		t.Fatalf("expected invalid origin 403, got %d", resp.Code)
	}

	req = httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(publishCall))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	req.Header.Set("Authorization", "Bearer "+token)
	resp = httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	if resp.Code != http.StatusForbidden {
		t.Fatalf("expected forbidden scope 403, got %d", resp.Code)
	}
	if !strings.Contains(resp.Header().Get("WWW-Authenticate"), `insufficient_scope`) {
		t.Fatalf("expected insufficient_scope header, got %s", resp.Header().Get("WWW-Authenticate"))
	}
}

func TestMCPHTTPCreateDraftRejectsExternalCoverImage(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "writer", []string{ScopeDraftWrite}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	payload := map[string]any{
		"jsonrpc": "2.0",
		"id":      1,
		"method":  "tools/call",
		"params": map[string]any{
			"name": "create_article_draft",
			"arguments": map[string]any{
				"title":       "Unsafe cover",
				"content":     "# body",
				"cover_image": "http://evil.example/x.jpg",
			},
		},
	}
	data, _ := json.Marshal(payload)
	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, bytes.NewReader(data))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	req.Header.Set("Authorization", "Bearer "+token)
	resp := httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	if resp.Code != http.StatusBadRequest {
		t.Fatalf("expected cover image reject 400, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestMCPHTTPCreateDraftRejectsTraversalCoverImage(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "writer-traversal", []string{ScopeDraftWrite}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	payload := `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_article_draft","arguments":{"title":"Unsafe cover","content":"# body","cover_image":"/uploads/../../evil.png"}}}`
	resp := performMCPHTTPCall(t, app, handler, token, payload)
	if resp.Code != http.StatusBadRequest {
		t.Fatalf("expected traversal cover image reject 400, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestMCPHTTPRejectsInvalidAcceptHeader(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "reader-accept", []string{ScopeBlogRead}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(`{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "text/plain")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	req.Header.Set("Authorization", "Bearer "+token)
	resp := httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	if resp.Code != http.StatusBadRequest {
		t.Fatalf("expected invalid accept 400, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestMCPHTTPPreviewMarkdownSanitizesXSS(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "previewer", []string{ScopeDraftWrite}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	payload := `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"preview_markdown","arguments":{"content":"# Hi\n<script>alert(1)</script>\n[link](javascript:alert(1))"}}}`
	resp := performMCPHTTPCall(t, app, handler, token, payload)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected preview 200, got %d: %s", resp.Code, resp.Body.String())
	}

	var envelope rpcResponseEnvelope
	if err := json.Unmarshal(resp.Body.Bytes(), &envelope); err != nil {
		t.Fatalf("decode preview response: %v", err)
	}
	var result struct {
		ContentHTML string `json:"content_html"`
	}
	if err := json.Unmarshal(envelope.Result, &result); err != nil {
		t.Fatalf("decode preview result: %v", err)
	}
	if strings.Contains(strings.ToLower(result.ContentHTML), "<script") || strings.Contains(strings.ToLower(result.ContentHTML), "javascript:") {
		t.Fatalf("unsafe markup leaked in preview: %s", result.ContentHTML)
	}
}

func TestMCPHTTPUploadRejectsOversizedPayload(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "uploader", []string{ScopeUpload}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	oversized := base64.StdEncoding.EncodeToString([]byte(strings.Repeat("a", int(app.Config.Upload.MaxSize)+1)))
	payload := map[string]any{
		"jsonrpc": "2.0",
		"id":      1,
		"method":  "tools/call",
		"params": map[string]any{
			"name": "upload_image",
			"arguments": map[string]any{
				"filename":       "x.png",
				"mime_type":      "image/png",
				"content_base64": oversized,
			},
		},
	}
	data, _ := json.Marshal(payload)
	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, bytes.NewReader(data))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	req.Header.Set("Authorization", "Bearer "+token)
	resp := httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	if resp.Code != http.StatusRequestEntityTooLarge {
		t.Fatalf("expected payload too large 413, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestMCPHTTPUploadRejectsDisguisedNonImage(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "uploader-disguised", []string{ScopeUpload}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	fakeImage := base64.StdEncoding.EncodeToString([]byte("not really an image"))
	payload := `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"upload_image","arguments":{"filename":"fake.png","mime_type":"image/png","content_base64":"` + fakeImage + `"}}}`
	resp := performMCPHTTPCall(t, app, handler, token, payload)
	if resp.Code != http.StatusUnsupportedMediaType {
		t.Fatalf("expected fake image reject 415, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestMCPHTTPReadArticleDoesNotLeakFuturePublish(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	future := time.Now().UTC().Add(24 * time.Hour)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Future article",
		Content:     "# body",
		Status:      "published",
		PublishedAt: &future,
	})
	_ = article

	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "reader", []string{ScopeBlogRead}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	payload := `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_article","arguments":{"slug":"future-article"}}}`
	req := httptest.NewRequest(http.MethodPost, app.Config.MCP.HTTPPath, strings.NewReader(payload))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("MCP-Protocol-Version", app.Config.MCP.ProtocolVersions[0])
	req.Header.Set("Origin", app.Config.MCP.AllowedOrigins[0])
	req.Header.Set("Authorization", "Bearer "+token)
	resp := httptest.NewRecorder()
	handler.ServeHTTP(resp, req)
	if resp.Code != http.StatusNotFound {
		t.Fatalf("expected future article hidden 404, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestMCPHTTPCreateUpdatePublishFlowMatchesPublicVisibility(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "editor", []string{ScopeDraftWrite, ScopePublish}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	createResp := performMCPHTTPCall(t, app, handler, token, `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_article_draft","arguments":{"title":"Draft Title","content":"# body"}}}`)
	if createResp.Code != http.StatusOK {
		t.Fatalf("expected create draft 200, got %d: %s", createResp.Code, createResp.Body.String())
	}
	var createEnvelope rpcResponseEnvelope
	if err := json.Unmarshal(createResp.Body.Bytes(), &createEnvelope); err != nil {
		t.Fatalf("decode create response: %v", err)
	}
	var createResult struct {
		ID   uint   `json:"id"`
		Slug string `json:"slug"`
	}
	if err := json.Unmarshal(createEnvelope.Result, &createResult); err != nil {
		t.Fatalf("decode create result: %v", err)
	}

	updatePayload := `{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"update_article","arguments":{"id":` + strconv.FormatUint(uint64(createResult.ID), 10) + `,"title":"Published Title"}}}`
	updateResp := performMCPHTTPCall(t, app, handler, token, updatePayload)
	if updateResp.Code != http.StatusOK {
		t.Fatalf("expected update draft 200, got %d: %s", updateResp.Code, updateResp.Body.String())
	}

	publishPayload := `{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"publish_article","arguments":{"id":` + strconv.FormatUint(uint64(createResult.ID), 10) + `}}}`
	publishResp := performMCPHTTPCall(t, app, handler, token, publishPayload)
	if publishResp.Code != http.StatusOK {
		t.Fatalf("expected publish 200, got %d: %s", publishResp.Code, publishResp.Body.String())
	}

	router := webhandler.NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	publicReq := httptest.NewRequest(http.MethodGet, "/api/articles/published-title", nil)
	publicResp := httptest.NewRecorder()
	router.ServeHTTP(publicResp, publicReq)
	if publicResp.Code != http.StatusOK {
		t.Fatalf("expected published article visible 200, got %d: %s", publicResp.Code, publicResp.Body.String())
	}

	oldSlugReq := httptest.NewRequest(http.MethodGet, "/api/articles/"+createResult.Slug, nil)
	oldSlugResp := httptest.NewRecorder()
	router.ServeHTTP(oldSlugResp, oldSlugReq)
	if oldSlugResp.Code != http.StatusMovedPermanently {
		t.Fatalf("expected historical slug redirect 301, got %d: %s", oldSlugResp.Code, oldSlugResp.Body.String())
	}
}

func TestMCPHTTPCreateDraftUsesExistingAdminAuthor(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	if err := app.DB.Unscoped().Where("username = ?", app.Config.Admin.InitUsername).Delete(&model.User{}).Error; err != nil {
		t.Fatalf("delete seeded admin: %v", err)
	}
	if err := app.DB.Create(&model.User{
		Username: "reader-one",
		Password: "hashed-reader-password",
		Role:     "user",
	}).Error; err != nil {
		t.Fatalf("seed reader: %v", err)
	}
	admin := model.User{
		Username: app.Config.Admin.InitUsername,
		Password: "hashed-admin-password",
		Role:     "admin",
	}
	if err := app.DB.Create(&admin).Error; err != nil {
		t.Fatalf("seed admin: %v", err)
	}
	if admin.ID == 1 {
		t.Fatalf("test setup expected admin id other than 1")
	}

	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	token, err := server.IssueToken(context.Background(), "writer-nondefault-admin", []string{ScopeDraftWrite}, "http")
	if err != nil {
		t.Fatalf("issue token: %v", err)
	}
	handler := server.HTTPHandler()

	createResp := performMCPHTTPCall(t, app, handler, token, `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_article_draft","arguments":{"title":"Admin Authored Draft","content":"# body"}}}`)
	if createResp.Code != http.StatusOK {
		t.Fatalf("expected create draft 200, got %d: %s", createResp.Code, createResp.Body.String())
	}

	var article model.Article
	if err := app.DB.Where("title = ?", "Admin Authored Draft").First(&article).Error; err != nil {
		t.Fatalf("query created article: %v", err)
	}
	if article.AuthorID != admin.ID {
		t.Fatalf("expected MCP draft author %d, got %d", admin.ID, article.AuthorID)
	}
}

func TestMCPHTTPRateLimitAppliesToReadAndUploadRequests(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	app.Config.MCP.RateLimit.ReadPerMinute = 1
	app.Config.MCP.RateLimit.UploadPer10Min = 1
	server := NewServer(app.Config, nil, app.DB, app.RateLimiter, app.Renderer, app.Categories, app.Articles, app.Uploads)
	readerToken, err := server.IssueToken(context.Background(), "reader-rate", []string{ScopeBlogRead}, "http")
	if err != nil {
		t.Fatalf("issue reader token: %v", err)
	}
	uploaderToken, err := server.IssueToken(context.Background(), "uploader-rate", []string{ScopeUpload}, "http")
	if err != nil {
		t.Fatalf("issue uploader token: %v", err)
	}
	handler := server.HTTPHandler()

	firstRead := performMCPHTTPCall(t, app, handler, readerToken, `{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}`)
	if firstRead.Code != http.StatusOK {
		t.Fatalf("expected first read 200, got %d: %s", firstRead.Code, firstRead.Body.String())
	}
	secondRead := performMCPHTTPCall(t, app, handler, readerToken, `{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}`)
	if secondRead.Code != http.StatusTooManyRequests {
		t.Fatalf("expected read rate limit 429, got %d: %s", secondRead.Code, secondRead.Body.String())
	}

	pngBase64 := testutil.MustPNGBase64(t)
	uploadPayload := `{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"upload_image","arguments":{"filename":"ok.png","mime_type":"image/png","content_base64":"` + pngBase64 + `"}}}`
	firstUpload := performMCPHTTPCall(t, app, handler, uploaderToken, uploadPayload)
	if firstUpload.Code != http.StatusOK {
		t.Fatalf("expected first upload 200, got %d: %s", firstUpload.Code, firstUpload.Body.String())
	}
	secondUpload := performMCPHTTPCall(t, app, handler, uploaderToken, uploadPayload)
	if secondUpload.Code != http.StatusTooManyRequests {
		t.Fatalf("expected upload rate limit 429, got %d: %s", secondUpload.Code, secondUpload.Body.String())
	}
}
