package mcp

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"blogWeb/internal/service"
)

func (s *Server) httpHandler() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != s.config.MCP.HTTPPath {
			http.NotFound(w, r)
			return
		}
		if r.Method == http.MethodGet {
			http.Error(w, http.StatusText(http.StatusMethodNotAllowed), http.StatusMethodNotAllowed)
			return
		}
		if r.Method != http.MethodPost {
			http.Error(w, http.StatusText(http.StatusMethodNotAllowed), http.StatusMethodNotAllowed)
			return
		}

		var request jsonRPCRequest
		if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
			writeMCPError(w, request.ID, &mcpError{Status: http.StatusBadRequest, Code: "invalid_params", Message: "JSON-RPC 请求格式错误"})
			return
		}

		requiredScope := requiredScopeForRequest(&request)
		client, authErr := s.authenticateHTTP(r.Context(), r, requiredScope)
		if authErr != nil {
			var clientID *uint
			if client != nil {
				clientID = &client.ID
			}
			s.writeAudit(r.Context(), clientID, "http", auditActionType(request.Method), auditTarget(request), requiredScope, "denied", requestID(request), clientIPFromRequest(r), authErr.Code, string(request.Params))
			s.writeAuthHeaders(w, authErr)
			writeMCPError(w, request.ID, authErr)
			return
		}
		if rateErr := s.enforceRateLimit(r.Context(), client.ID, &request); rateErr != nil {
			s.writeAudit(r.Context(), &client.ID, "http", auditActionType(request.Method), auditTarget(request), requiredScope, "denied", requestID(request), clientIPFromRequest(r), rateErr.Code, string(request.Params))
			writeMCPError(w, request.ID, rateErr)
			return
		}

		result, err := s.dispatchRPC(r.Context(), &request)
		if err != nil {
			if appErr, ok := service.IsAppError(err); ok {
				s.writeAudit(r.Context(), &client.ID, "http", auditActionType(request.Method), auditTarget(request), requiredScope, "error", requestID(request), clientIPFromRequest(r), appErr.Code, string(request.Params))
				writeMCPError(w, request.ID, &mcpError{Status: appErr.StatusCode, Code: appErr.Code, Message: appErr.Message})
				return
			}
			s.writeAudit(r.Context(), &client.ID, "http", auditActionType(request.Method), auditTarget(request), requiredScope, "error", requestID(request), clientIPFromRequest(r), "internal_error", string(request.Params))
			writeMCPError(w, request.ID, &mcpError{Status: http.StatusInternalServerError, Code: "internal_error", Message: err.Error()})
			return
		}

		s.writeAudit(r.Context(), &client.ID, "http", auditActionType(request.Method), auditTarget(request), requiredScope, "success", requestID(request), clientIPFromRequest(r), "", string(request.Params))

		response := jsonRPCResponse{
			JSONRPC: "2.0",
			ID:      request.ID,
			Result:  result,
		}
		if request.ID == nil {
			w.WriteHeader(http.StatusAccepted)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(response)
	})
}

func requiredScopeForRequest(request *jsonRPCRequest) string {
	switch request.Method {
	case "resources/read":
		var params struct {
			URI string `json:"uri"`
		}
		if err := json.Unmarshal(request.Params, &params); err != nil {
			return ""
		}
		switch {
		case params.URI == "blog://site/meta":
			return ScopeBlogRead
		case params.URI == "blog://categories":
			return ScopeCategoryRead
		case strings.HasPrefix(params.URI, "blog://articles/"):
			return ScopeBlogRead
		case strings.HasPrefix(params.URI, "blog://drafts/"):
			return ScopeDraftWrite
		case strings.HasPrefix(params.URI, "blog://categories/") && strings.HasSuffix(params.URI, "/articles"):
			return ScopeBlogRead
		default:
			return ""
		}
	case "tools/call":
		var params struct {
			Name string `json:"name"`
		}
		if err := json.Unmarshal(request.Params, &params); err != nil {
			return ""
		}
		switch params.Name {
		case "list_articles", "get_article":
			return ScopeBlogRead
		case "list_categories":
			return ScopeCategoryRead
		case "preview_markdown", "create_article_draft", "update_article":
			return ScopeDraftWrite
		case "publish_article", "unpublish_article":
			return ScopePublish
		case "upload_image":
			return ScopeUpload
		case "create_category", "update_category":
			return ScopeCategoryWrite
		default:
			return ""
		}
	default:
		return ""
	}
}

func auditActionType(method string) string {
	switch method {
	case "resources/read":
		return "resource_read"
	case "prompts/get":
		return "prompt_get"
	default:
		return "tool_call"
	}
}

func auditTarget(request jsonRPCRequest) string {
	switch request.Method {
	case "tools/call":
		var params struct {
			Name string `json:"name"`
		}
		if err := json.Unmarshal(request.Params, &params); err == nil && strings.TrimSpace(params.Name) != "" {
			return params.Name
		}
	case "resources/read":
		var params struct {
			URI string `json:"uri"`
		}
		if err := json.Unmarshal(request.Params, &params); err == nil && strings.TrimSpace(params.URI) != "" {
			return params.URI
		}
	case "prompts/get":
		var params struct {
			Name string `json:"name"`
		}
		if err := json.Unmarshal(request.Params, &params); err == nil && strings.TrimSpace(params.Name) != "" {
			return params.Name
		}
	}
	return request.Method
}

func requestID(request jsonRPCRequest) string {
	if request.ID == nil {
		return ""
	}
	return strings.TrimSpace(fmt.Sprint(request.ID))
}

func clientIPFromRequest(r *http.Request) string {
	return r.RemoteAddr
}

func writeMCPError(w http.ResponseWriter, id any, err *mcpError) {
	status := http.StatusInternalServerError
	code := "internal_error"
	message := "服务端错误"
	if err != nil {
		status = err.Status
		code = err.Code
		message = err.Message
	}
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(jsonRPCResponse{
		JSONRPC: "2.0",
		ID:      id,
		Error: &jsonRPCError{
			Code:    status,
			Message: message,
			Data: map[string]any{
				"code":       code,
				"message":    message,
				"request_id": requestID(jsonRPCRequest{ID: id}),
			},
		},
	})
}
