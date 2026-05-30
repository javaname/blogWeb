package mcp

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"crypto/subtle"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"strings"
	"time"

	"blogWeb/config"
	"blogWeb/internal/model"
	"blogWeb/internal/service"

	"gorm.io/gorm"
)

type Server struct {
	config     *config.Config
	logger     *slog.Logger
	db         *gorm.DB
	limiter    *service.RateLimiter
	categories *service.CategoryService
	articles   *service.ArticleService
	uploads    *service.UploadService
}

func NewServer(
	cfg *config.Config,
	logger *slog.Logger,
	db *gorm.DB,
	limiter *service.RateLimiter,
	renderer *service.RendererService,
	categories *service.CategoryService,
	articles *service.ArticleService,
	uploads *service.UploadService,
) *Server {
	return &Server{
		config:     cfg,
		logger:     logger,
		db:         db,
		limiter:    limiter,
		categories: categories,
		articles:   articles,
		uploads:    uploads,
	}
}

func (s *Server) HTTPHandler() http.Handler {
	return s.httpHandler()
}

func (s *Server) ServeStdio(ctx context.Context, input io.Reader, output io.Writer, stderr io.Writer) error {
	return s.serveStdio(ctx, input, output, stderr)
}

func (s *Server) IssueToken(ctx context.Context, name string, scopes []string, transport string) (string, error) {
	name = strings.TrimSpace(name)
	if name == "" {
		return "", errors.New("client name is required")
	}
	normalizedScopes := normalizeScopes(scopes)
	if len(normalizedScopes) == 0 {
		return "", errors.New("at least one scope is required")
	}
	if transport == "" {
		transport = "http"
	}

	token, err := service.NewToken(32)
	if err != nil {
		return "", err
	}
	hash := s.tokenHash(token)
	scopeJSON, _ := json.Marshal(normalizedScopes)

	client := &model.MCPClient{
		Name:      name,
		TokenHash: hash,
		Scopes:    string(scopeJSON),
		Transport: transport,
		IsEnabled: true,
	}

	if err := s.db.WithContext(ctx).
		Where("name = ?", name).
		Assign(map[string]any{
			"token_hash":   hash,
			"scopes":       client.Scopes,
			"transport":    transport,
			"is_enabled":   true,
			"updated_at":   time.Now().UTC(),
			"last_used_at": sql.NullTime{},
		}).
		FirstOrCreate(client).Error; err != nil {
		return "", err
	}
	return token, nil
}

func (s *Server) RevokeToken(ctx context.Context, name string) error {
	if strings.TrimSpace(name) == "" {
		return errors.New("client name is required")
	}
	return s.db.WithContext(ctx).Model(&model.MCPClient{}).Where("name = ?", name).Update("is_enabled", false).Error
}

func (s *Server) tokenHash(token string) string {
	key := []byte(s.config.Session.Secret)
	mac := hmac.New(sha256.New, key)
	_, _ = mac.Write([]byte(token))
	return hex.EncodeToString(mac.Sum(nil))
}

func (s *Server) authenticateHTTP(ctx context.Context, r *http.Request, requiredScope string) (*model.MCPClient, *mcpError) {
	if !containsString(s.config.MCP.ProtocolVersions, r.Header.Get("MCP-Protocol-Version")) {
		return nil, &mcpError{Status: http.StatusBadRequest, Code: "invalid_params", Message: "不支持的 MCP 协议版本"}
	}
	if contentType := r.Header.Get("Content-Type"); !strings.Contains(contentType, "application/json") {
		return nil, &mcpError{Status: http.StatusUnsupportedMediaType, Code: "invalid_params", Message: "Content-Type 必须为 application/json"}
	}
	if accept := r.Header.Get("Accept"); accept != "" && !strings.Contains(accept, "application/json") {
		return nil, &mcpError{Status: http.StatusBadRequest, Code: "invalid_params", Message: "Accept 必须包含 application/json"}
	}
	if s.config.MCP.RequireOriginCheck {
		origin := strings.TrimSpace(r.Header.Get("Origin"))
		if origin == "" || !containsString(s.config.MCP.AllowedOrigins, origin) {
			return nil, &mcpError{Status: http.StatusForbidden, Code: "invalid_origin", Message: "Origin 不允许"}
		}
	}

	authHeader := strings.TrimSpace(r.Header.Get("Authorization"))
	if !strings.HasPrefix(authHeader, "Bearer ") {
		return nil, &mcpError{Status: http.StatusUnauthorized, Code: "auth_required", Message: "缺少 Bearer Token"}
	}
	token := strings.TrimSpace(strings.TrimPrefix(authHeader, "Bearer "))
	if token == "" {
		return nil, &mcpError{Status: http.StatusUnauthorized, Code: "auth_required", Message: "缺少 Bearer Token"}
	}

	var clients []model.MCPClient
	if err := s.db.WithContext(ctx).Where("is_enabled = ?", true).Find(&clients).Error; err != nil {
		return nil, &mcpError{Status: http.StatusInternalServerError, Code: "internal_error", Message: err.Error()}
	}
	tokenHash := s.tokenHash(token)
	var matched *model.MCPClient
	for index := range clients {
		client := &clients[index]
		if subtle.ConstantTimeCompare([]byte(client.TokenHash), []byte(tokenHash)) == 1 {
			matched = client
			break
		}
	}
	if matched == nil {
		return nil, &mcpError{Status: http.StatusUnauthorized, Code: "invalid_token", Message: "token 无效或已撤销"}
	}

	scopes := parseScopes(matched.Scopes)
	if requiredScope != "" && !hasScope(scopes, requiredScope) {
		return matched, &mcpError{Status: http.StatusForbidden, Code: "forbidden_scope", Message: fmt.Sprintf("MCP token 缺少 %s 权限", requiredScope), Scope: requiredScope}
	}

	_ = s.db.WithContext(ctx).Model(&model.MCPClient{}).Where("id = ?", matched.ID).Update("last_used_at", time.Now().UTC()).Error
	return matched, nil
}

func (s *Server) enforceRateLimit(ctx context.Context, clientID uint, request *jsonRPCRequest) *mcpError {
	if s.limiter == nil || request == nil {
		return nil
	}

	var (
		key        string
		max        int
		window     time.Duration
		actionType = auditActionType(request.Method)
	)

	switch {
	case request.Method == "resources/read":
		key = service.MCPReadRateKey(clientID)
		max = s.config.MCP.RateLimit.ReadPerMinute
		window = time.Minute
	case request.Method == "tools/call":
		var params struct {
			Name string `json:"name"`
		}
		if err := json.Unmarshal(request.Params, &params); err != nil {
			return nil
		}
		switch params.Name {
		case "upload_image":
			key = service.MCPUploadRateKey(clientID)
			max = s.config.MCP.RateLimit.UploadPer10Min
			window = 10 * time.Minute
		case "publish_article", "unpublish_article":
			key = service.MCPWriteRateKey(clientID) + ":publish"
			max = s.config.MCP.RateLimit.PublishPer10Min
			window = 10 * time.Minute
		case "create_article_draft", "update_article", "create_category", "update_category":
			key = service.MCPWriteRateKey(clientID)
			max = s.config.MCP.RateLimit.WritePerMinute
			window = time.Minute
		default:
			if isWriteTool(params.Name) {
				key = service.MCPWriteRateKey(clientID)
				max = s.config.MCP.RateLimit.WritePerMinute
				window = time.Minute
			} else {
				key = service.MCPReadRateKey(clientID)
				max = s.config.MCP.RateLimit.ReadPerMinute
				window = time.Minute
			}
		}
	case request.Method == "prompts/get", request.Method == "tools/list", request.Method == "resources/list", request.Method == "initialize", request.Method == "prompts/list":
		key = service.MCPReadRateKey(clientID)
		max = s.config.MCP.RateLimit.ReadPerMinute
		window = time.Minute
	default:
		return nil
	}

	if max <= 0 || window <= 0 {
		return nil
	}

	allowed, _, err := s.limiter.Allow(ctx, key, max, window)
	if err != nil {
		return &mcpError{Status: http.StatusInternalServerError, Code: "internal_error", Message: err.Error()}
	}
	if !allowed {
		_ = actionType
		return &mcpError{Status: http.StatusTooManyRequests, Code: "rate_limited", Message: "MCP 请求过于频繁，请稍后再试"}
	}
	return nil
}
