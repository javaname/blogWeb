package service

import (
	"context"
	"strings"

	"blogWeb/internal/model"

	"gorm.io/gorm"
)

type CategoryWithCount struct {
	model.Category
	ArticleCount int64 `json:"article_count"`
}

type CreateCategoryInput struct {
	Name      string `json:"name"`
	Slug      string `json:"slug"`
	SortOrder int    `json:"sort_order"`
}

type UpdateCategoryInput struct {
	Name      *string `json:"name"`
	Slug      *string `json:"slug"`
	SortOrder *int    `json:"sort_order"`
}

type CategoryService struct {
	db       *gorm.DB
	renderer *RendererService
}

func NewCategoryService(db *gorm.DB, renderer *RendererService) *CategoryService {
	return &CategoryService{db: db, renderer: renderer}
}

func (s *CategoryService) List(ctx context.Context) ([]CategoryWithCount, error) {
	var categories []CategoryWithCount
	if err := s.db.WithContext(ctx).
		Table("categories").
		Select("categories.*, COUNT(articles.id) AS article_count").
		Joins("LEFT JOIN articles ON articles.category_id = categories.id").
		Group("categories.id").
		Order("categories.sort_order ASC, categories.id ASC").
		Scan(&categories).Error; err != nil {
		return nil, err
	}
	return categories, nil
}

func (s *CategoryService) Create(ctx context.Context, input CreateCategoryInput) (*model.Category, error) {
	name := strings.TrimSpace(input.Name)
	if name == "" || len([]rune(name)) > 40 {
		return nil, NewAppError(400, "invalid_params", "分类名称长度需为 1-40 字符")
	}

	slug := strings.TrimSpace(strings.ToLower(input.Slug))
	if slug == "" {
		slug = Slugify(name)
	}
	if !IsValidSlug(slug) {
		return nil, NewAppError(400, "invalid_params", "分类 slug 不合法")
	}

	category := &model.Category{
		Name:      name,
		Slug:      slug,
		SortOrder: input.SortOrder,
	}
	if err := s.db.WithContext(ctx).Create(category).Error; err != nil {
		return nil, NewAppError(409, "conflict", "分类名称或 slug 已存在")
	}
	return category, nil
}

func (s *CategoryService) Update(ctx context.Context, id uint, input UpdateCategoryInput) (*model.Category, error) {
	var category model.Category
	if err := s.db.WithContext(ctx).First(&category, id).Error; err != nil {
		return nil, NewAppError(404, "not_found", "分类不存在")
	}

	if input.Name != nil {
		name := strings.TrimSpace(*input.Name)
		if name == "" || len([]rune(name)) > 40 {
			return nil, NewAppError(400, "invalid_params", "分类名称长度需为 1-40 字符")
		}
		category.Name = name
	}
	if input.Slug != nil {
		slug := strings.TrimSpace(strings.ToLower(*input.Slug))
		if slug == "" {
			slug = Slugify(category.Name)
		}
		if !IsValidSlug(slug) {
			return nil, NewAppError(400, "invalid_params", "分类 slug 不合法")
		}
		category.Slug = slug
	}
	if input.SortOrder != nil {
		if *input.SortOrder < 0 {
			return nil, NewAppError(400, "invalid_params", "sort_order 不能为负数")
		}
		category.SortOrder = *input.SortOrder
	}

	if err := s.db.WithContext(ctx).Save(&category).Error; err != nil {
		return nil, NewAppError(409, "conflict", "分类名称或 slug 已存在")
	}
	return &category, nil
}

func (s *CategoryService) Delete(ctx context.Context, id uint) error {
	var category model.Category
	if err := s.db.WithContext(ctx).First(&category, id).Error; err != nil {
		return NewAppError(404, "not_found", "分类不存在")
	}

	var publishedCount int64
	if err := s.db.WithContext(ctx).
		Model(&model.Article{}).
		Where("category_id = ? AND status = ?", id, "published").
		Count(&publishedCount).Error; err != nil {
		return err
	}
	if publishedCount > 0 {
		return NewAppError(409, "conflict", "该分类下存在已发布文章，无法删除")
	}

	return s.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		if err := tx.Model(&model.Article{}).Where("category_id = ?", id).Update("category_id", nil).Error; err != nil {
			return err
		}
		return tx.Delete(&category).Error
	})
}

func (s *CategoryService) Sort(ctx context.Context, ids []uint) error {
	return s.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		for index, id := range ids {
			if err := tx.Model(&model.Category{}).Where("id = ?", id).Update("sort_order", index).Error; err != nil {
				return err
			}
		}
		return nil
	})
}

func (s *CategoryService) GetBySlug(ctx context.Context, slug string) (*model.Category, error) {
	var category model.Category
	if err := s.db.WithContext(ctx).Where("slug = ?", slug).First(&category).Error; err != nil {
		return nil, NewAppError(404, "not_found", "分类不存在")
	}
	return &category, nil
}
