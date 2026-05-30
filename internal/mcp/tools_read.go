package mcp

import (
	"context"
	"encoding/json"

	"blogWeb/internal/service"
)

func (s *Server) callReadTool(ctx context.Context, name string, params json.RawMessage) (any, string, error) {
	switch name {
	case "list_articles":
		var request struct {
			Cursor   string `json:"cursor"`
			Category string `json:"category"`
			Limit    int    `json:"limit"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopeBlogRead, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		if request.Category != "" {
			if err := validateSlug(request.Category); err != nil {
				return nil, ScopeBlogRead, err
			}
		}
		result, err := s.articles.ListPublished(ctx, service.ListPublishedInput{
			Cursor:       request.Cursor,
			CategorySlug: request.Category,
			Limit:        request.Limit,
		})
		return result, ScopeBlogRead, err
	case "get_article":
		var request struct {
			Slug string `json:"slug"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopeBlogRead, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		if err := validateSlug(request.Slug); err != nil {
			return nil, ScopeBlogRead, err
		}
		result, _, err := s.articles.GetPublishedBySlug(ctx, request.Slug, "")
		return result, ScopeBlogRead, err
	case "list_categories":
		list, err := s.categories.List(ctx)
		if err != nil {
			return nil, ScopeCategoryRead, err
		}
		return map[string]any{"list": list}, ScopeCategoryRead, nil
	case "preview_markdown":
		var request struct {
			Content string `json:"content"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		if err := validateMarkdown(request.Content); err != nil {
			return nil, ScopeDraftWrite, err
		}
		contentHTML, excerpt, err := s.articles.PreviewMarkdown(request.Content)
		if err != nil {
			return nil, ScopeDraftWrite, err
		}
		return map[string]any{"content_html": contentHTML, "excerpt": excerpt}, ScopeDraftWrite, nil
	default:
		return nil, "", service.NewAppError(404, "not_found", "工具不存在")
	}
}
