package mcp

import (
	"context"
	"strconv"
	"strings"

	"blogWeb/internal/service"
)

func (s *Server) readResource(ctx context.Context, uri string) (any, string, error) {
	switch {
	case uri == "blog://site/meta":
		return map[string]any{
			"title":       s.config.Site.Title,
			"description": s.config.Site.Description,
			"base_url":    s.config.Site.BaseURL,
			"version":     "v6",
		}, ScopeBlogRead, nil
	case uri == "blog://categories":
		list, err := s.categories.List(ctx)
		if err != nil {
			return nil, ScopeCategoryRead, err
		}
		return map[string]any{"list": list}, ScopeCategoryRead, nil
	case strings.HasPrefix(uri, "blog://articles/"):
		slug := strings.TrimPrefix(uri, "blog://articles/")
		if err := validateSlug(slug); err != nil {
			return nil, ScopeBlogRead, err
		}
		result, _, err := s.articles.GetPublishedBySlug(ctx, slug, "")
		if err != nil {
			return nil, ScopeBlogRead, err
		}
		return map[string]any{
			"id":           result.ID,
			"title":        result.Title,
			"slug":         result.Slug,
			"content_html": result.ContentHTML,
			"excerpt":      result.Excerpt,
			"category":     result.Category,
			"is_pinned":    result.IsPinned,
			"published_at": result.PublishedAt,
			"updated_at":   result.UpdatedAt,
		}, ScopeBlogRead, nil
	case strings.HasPrefix(uri, "blog://drafts/"):
		id, err := strconv.ParseUint(strings.TrimPrefix(uri, "blog://drafts/"), 10, 64)
		if err != nil || id == 0 {
			return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "草稿 ID 非法")
		}
		result, err := s.articles.GetByID(ctx, uint(id))
		if err != nil {
			return nil, ScopeDraftWrite, err
		}
		return result, ScopeDraftWrite, nil
	case strings.HasPrefix(uri, "blog://categories/") && strings.HasSuffix(uri, "/articles"):
		middle := strings.TrimSuffix(strings.TrimPrefix(uri, "blog://categories/"), "/articles")
		if err := validateSlug(middle); err != nil {
			return nil, ScopeBlogRead, err
		}
		category, err := s.categories.GetBySlug(ctx, middle)
		if err != nil {
			return nil, ScopeBlogRead, err
		}
		result, err := s.articles.ListPublished(ctx, service.ListPublishedInput{
			CategorySlug: middle,
			Limit:        50,
		})
		if err != nil {
			return nil, ScopeBlogRead, err
		}
		return map[string]any{
			"category": map[string]any{
				"id":   category.ID,
				"name": category.Name,
				"slug": category.Slug,
			},
			"list": result.List,
		}, ScopeBlogRead, nil
	default:
		return nil, "", service.NewAppError(404, "not_found", "资源不存在")
	}
}
