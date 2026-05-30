package mcp

import (
	"context"
	"encoding/json"
	"errors"
	"strings"

	"blogWeb/internal/model"
	"blogWeb/internal/service"

	"gorm.io/gorm"
)

func (s *Server) defaultAuthorID(ctx context.Context) (uint, error) {
	var user model.User
	username := strings.TrimSpace(s.config.Admin.InitUsername)
	if username != "" {
		err := s.db.WithContext(ctx).
			Where("username = ? AND role = ?", username, "admin").
			First(&user).Error
		if err == nil {
			return user.ID, nil
		}
		if !errors.Is(err, gorm.ErrRecordNotFound) {
			return 0, err
		}
	}

	err := s.db.WithContext(ctx).
		Where("role = ?", "admin").
		Order("id ASC").
		First(&user).Error
	if err == nil {
		return user.ID, nil
	}
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return 0, service.NewAppError(500, "admin_not_configured", "未找到可用于创建文章的管理员账号")
	}
	return 0, err
}

func (s *Server) callWriteTool(ctx context.Context, name string, params json.RawMessage) (any, string, error) {
	switch name {
	case "create_article_draft":
		var request struct {
			Title      string `json:"title"`
			Content    string `json:"content"`
			CategoryID *uint  `json:"category_id"`
			CoverImage string `json:"cover_image"`
			IsPinned   bool   `json:"is_pinned"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		if err := validateTitle(request.Title); err != nil {
			return nil, ScopeDraftWrite, err
		}
		if err := validateMarkdown(request.Content); err != nil {
			return nil, ScopeDraftWrite, err
		}
		if err := validateCoverImage(request.CoverImage); err != nil {
			return nil, ScopeDraftWrite, err
		}
		authorID, err := s.defaultAuthorID(ctx)
		if err != nil {
			return nil, ScopeDraftWrite, err
		}
		article, err := s.articles.Create(ctx, service.CreateArticleInput{
			Title:      request.Title,
			Content:    request.Content,
			CategoryID: request.CategoryID,
			CoverImage: request.CoverImage,
			IsPinned:   request.IsPinned,
			Status:     "draft",
			AuthorID:   authorID,
		})
		if err != nil {
			return nil, ScopeDraftWrite, err
		}
		return map[string]any{"id": article.ID, "slug": article.Slug, "status": article.Status}, ScopeDraftWrite, nil
	case "update_article":
		var raw map[string]json.RawMessage
		if err := json.Unmarshal(params, &raw); err != nil {
			return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		var request struct {
			ID uint `json:"id"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		input := service.UpdateArticleInput{}
		if value, ok := raw["title"]; ok {
			var title string
			if err := json.Unmarshal(value, &title); err != nil {
				return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "title 类型错误")
			}
			if err := validateTitle(title); err != nil {
				return nil, ScopeDraftWrite, err
			}
			input.Title = &title
		}
		if value, ok := raw["content"]; ok {
			var content string
			if err := json.Unmarshal(value, &content); err != nil {
				return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "content 类型错误")
			}
			if err := validateMarkdown(content); err != nil {
				return nil, ScopeDraftWrite, err
			}
			input.Content = &content
		}
		if value, ok := raw["cover_image"]; ok {
			var coverImage string
			if err := json.Unmarshal(value, &coverImage); err != nil {
				return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "cover_image 类型错误")
			}
			if err := validateCoverImage(coverImage); err != nil {
				return nil, ScopeDraftWrite, err
			}
			input.CoverImage = &coverImage
		}
		if value, ok := raw["category_id"]; ok {
			if string(value) == "null" {
				input.ClearCategory = true
			} else {
				var categoryID uint
				if err := json.Unmarshal(value, &categoryID); err != nil {
					return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "category_id 类型错误")
				}
				input.CategoryID = &categoryID
			}
		}
		if value, ok := raw["is_pinned"]; ok {
			var pinned bool
			if err := json.Unmarshal(value, &pinned); err != nil {
				return nil, ScopeDraftWrite, service.NewAppError(400, "invalid_params", "is_pinned 类型错误")
			}
			input.IsPinned = &pinned
		}
		article, err := s.articles.Update(ctx, request.ID, input)
		if err != nil {
			return nil, ScopeDraftWrite, err
		}
		return map[string]any{"id": article.ID, "slug": article.Slug, "updated_at": article.UpdatedAt}, ScopeDraftWrite, nil
	case "publish_article":
		var request struct {
			ID          uint   `json:"id"`
			PublishedAt string `json:"published_at"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopePublish, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		publishedAt, err := parseRFC3339(request.PublishedAt)
		if err != nil {
			return nil, ScopePublish, err
		}
		article, err := s.articles.UpdateStatus(ctx, request.ID, service.UpdateStatusInput{
			Status:      "published",
			PublishedAt: publishedAt,
		})
		if err != nil {
			return nil, ScopePublish, err
		}
		return map[string]any{"id": article.ID, "status": article.Status, "published_at": article.PublishedAt}, ScopePublish, nil
	case "unpublish_article":
		var request struct {
			ID uint `json:"id"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopePublish, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		article, err := s.articles.UpdateStatus(ctx, request.ID, service.UpdateStatusInput{Status: "draft"})
		if err != nil {
			return nil, ScopePublish, err
		}
		return map[string]any{"id": article.ID, "status": article.Status}, ScopePublish, nil
	case "upload_image":
		var request struct {
			Filename      string `json:"filename"`
			MIMEType      string `json:"mime_type"`
			ContentBase64 string `json:"content_base64"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopeUpload, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		if err := validateBase64Size(request.ContentBase64, s.config.Upload.MaxSize); err != nil {
			return nil, ScopeUpload, err
		}
		result, err := s.uploads.StoreBase64(request.Filename, request.ContentBase64)
		if err != nil {
			return nil, ScopeUpload, err
		}
		return result, ScopeUpload, nil
	case "create_category":
		var request struct {
			Name      string `json:"name"`
			Slug      string `json:"slug"`
			SortOrder int    `json:"sort_order"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopeCategoryWrite, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		if err := validateCategoryName(request.Name); err != nil {
			return nil, ScopeCategoryWrite, err
		}
		if request.Slug != "" {
			if err := validateSlug(request.Slug); err != nil {
				return nil, ScopeCategoryWrite, err
			}
		}
		category, err := s.categories.Create(ctx, service.CreateCategoryInput{
			Name:      request.Name,
			Slug:      request.Slug,
			SortOrder: request.SortOrder,
		})
		if err != nil {
			return nil, ScopeCategoryWrite, err
		}
		return map[string]any{"id": category.ID, "name": category.Name, "slug": category.Slug}, ScopeCategoryWrite, nil
	case "update_category":
		var request struct {
			ID        uint    `json:"id"`
			Name      *string `json:"name"`
			Slug      *string `json:"slug"`
			SortOrder *int    `json:"sort_order"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ScopeCategoryWrite, service.NewAppError(400, "invalid_params", "tool 参数格式错误")
		}
		if request.Name != nil {
			if err := validateCategoryName(*request.Name); err != nil {
				return nil, ScopeCategoryWrite, err
			}
		}
		if request.Slug != nil {
			if err := validateSlug(*request.Slug); err != nil {
				return nil, ScopeCategoryWrite, err
			}
		}
		category, err := s.categories.Update(ctx, request.ID, service.UpdateCategoryInput{
			Name:      request.Name,
			Slug:      request.Slug,
			SortOrder: request.SortOrder,
		})
		if err != nil {
			return nil, ScopeCategoryWrite, err
		}
		return map[string]any{"id": category.ID, "name": category.Name, "slug": category.Slug, "sort_order": category.SortOrder}, ScopeCategoryWrite, nil
	default:
		return nil, "", service.NewAppError(404, "not_found", "工具不存在")
	}
}
