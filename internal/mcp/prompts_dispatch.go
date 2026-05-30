package mcp

import (
	"context"
	"encoding/json"
	"strings"

	"blogWeb/internal/service"
)

func (s *Server) dispatchRPC(ctx context.Context, request *jsonRPCRequest) (any, error) {
	return s.dispatchRPCWithAccess(ctx, request, true)
}

func (s *Server) dispatchRPCWithAccess(ctx context.Context, request *jsonRPCRequest, allowWrites bool) (any, error) {
	switch request.Method {
	case "initialize":
		return map[string]any{
			"serverInfo": map[string]any{
				"name":    "blogWeb",
				"version": "v6",
			},
			"capabilities": map[string]any{
				"resources": map[string]any{
					"listChanged": false,
				},
				"tools":   map[string]any{},
				"prompts": map[string]any{},
			},
			"resources": s.resourceTemplates(allowWrites),
		}, nil
	case "resources/list":
		return map[string]any{"resources": s.resourceTemplates(allowWrites)}, nil
	case "resources/read":
		var requestParams struct {
			URI string `json:"uri"`
		}
		if err := json.Unmarshal(request.Params, &requestParams); err != nil {
			return nil, service.NewAppError(400, "invalid_params", "resource 参数格式错误")
		}
		if !allowWrites && strings.HasPrefix(requestParams.URI, "blog://drafts/") {
			return nil, service.NewAppError(403, "forbidden_scope", "stdio 写能力默认关闭，请开启 mcp.stdio_write_enabled")
		}
		result, _, err := s.readResource(ctx, requestParams.URI)
		return result, err
	case "tools/list":
		return map[string]any{"tools": toolsCatalog(allowWrites)}, nil
	case "tools/call":
		var toolRequest struct {
			Name      string          `json:"name"`
			Arguments json.RawMessage `json:"arguments"`
		}
		if err := json.Unmarshal(request.Params, &toolRequest); err != nil {
			return nil, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		if isWriteTool(toolRequest.Name) {
			if !allowWrites {
				return nil, service.NewAppError(403, "forbidden_scope", "stdio 写能力默认关闭，请开启 mcp.stdio_write_enabled")
			}
			result, _, err := s.callWriteTool(ctx, toolRequest.Name, toolRequest.Arguments)
			return result, err
		}
		result, _, err := s.callReadTool(ctx, toolRequest.Name, toolRequest.Arguments)
		return result, err
	case "prompts/list":
		return map[string]any{"prompts": promptsCatalog()}, nil
	case "prompts/get":
		var promptRequest struct {
			Name      string          `json:"name"`
			Arguments json.RawMessage `json:"arguments"`
		}
		if err := json.Unmarshal(request.Params, &promptRequest); err != nil {
			return nil, service.NewAppError(400, "invalid_params", "prompt 参数格式错误")
		}
		return s.getPrompt(ctx, promptRequest.Name, promptRequest.Arguments)
	default:
		return nil, service.NewAppError(404, "not_found", "不支持的方法")
	}
}

func toolsCatalog(includeWrites bool) []map[string]any {
	tools := []map[string]any{
		{"name": "list_articles"},
		{"name": "get_article"},
		{"name": "list_categories"},
		{"name": "preview_markdown"},
	}
	if includeWrites {
		tools = append(tools,
			map[string]any{"name": "create_article_draft"},
			map[string]any{"name": "update_article"},
			map[string]any{"name": "publish_article"},
			map[string]any{"name": "unpublish_article"},
			map[string]any{"name": "upload_image"},
			map[string]any{"name": "create_category"},
			map[string]any{"name": "update_category"},
		)
	}
	return tools
}

func promptsCatalog() []map[string]any {
	return []map[string]any{
		{"name": "draft_article_from_outline"},
		{"name": "seo_review_article"},
		{"name": "rewrite_article_summary"},
	}
}

func isWriteTool(name string) bool {
	switch name {
	case "create_article_draft", "update_article", "publish_article", "unpublish_article", "upload_image", "create_category", "update_category":
		return true
	default:
		return false
	}
}
