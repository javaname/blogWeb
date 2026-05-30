package service

import (
	"context"
	"errors"
	"fmt"
	"strings"
	"time"

	"blogWeb/internal/model"

	"gorm.io/gorm"
)

type PublicArticleSummary struct {
	ID          uint            `json:"id"`
	Title       string          `json:"title"`
	Slug        string          `json:"slug"`
	CoverImage  string          `json:"cover_image"`
	Excerpt     string          `json:"excerpt"`
	Category    *PublicCategory `json:"category,omitempty"`
	Author      *PublicAuthor   `json:"author,omitempty"`
	IsPinned    bool            `json:"is_pinned"`
	LikeCount   int64           `json:"like_count"`
	ReadTimeMin int             `json:"read_time_min"`
	PublishedAt *time.Time      `json:"published_at"`
}

type PublicCategory struct {
	ID   uint   `json:"id"`
	Name string `json:"name"`
	Slug string `json:"slug"`
}

type PublicAuthor struct {
	ID       uint   `json:"id"`
	Username string `json:"username"`
}

type PublicArticleDetail struct {
	ID             uint            `json:"id"`
	Title          string          `json:"title"`
	Slug           string          `json:"slug"`
	ContentHTML    string          `json:"content_html"`
	CoverImage     string          `json:"cover_image"`
	Excerpt        string          `json:"excerpt"`
	Category       *PublicCategory `json:"category,omitempty"`
	Author         *PublicAuthor   `json:"author,omitempty"`
	IsPinned       bool            `json:"is_pinned"`
	LikeCount      int64           `json:"like_count"`
	UserLiked      bool            `json:"user_liked"`
	UserBookmarked bool            `json:"user_bookmarked"`
	AuthorFollowed bool            `json:"author_followed"`
	ReadTimeMin    int             `json:"read_time_min"`
	PublishedAt    *time.Time      `json:"published_at"`
	CreatedAt      time.Time       `json:"created_at"`
	UpdatedAt      time.Time       `json:"updated_at"`
}

type ListPublishedInput struct {
	Cursor       string
	CategorySlug string
	Keyword      string
	Limit        int
}

type ListPublishedResult struct {
	List       []PublicArticleSummary `json:"list"`
	NextCursor string                 `json:"next_cursor"`
	HasMore    bool                   `json:"has_more"`
}

type ListAdminInput struct {
	Page       int
	PageSize   int
	Status     string
	CategoryID uint
	Keyword    string
	SortBy     string
	SortOrder  string
}

type AdminArticleSummary struct {
	ID          uint              `json:"id"`
	Title       string            `json:"title"`
	Slug        string            `json:"slug"`
	CoverImage  string            `json:"cover_image"`
	Status      string            `json:"status"`
	IsPinned    bool              `json:"is_pinned"`
	Category    *AdminCategoryRef `json:"category,omitempty"`
	Author      *AdminAuthorRef   `json:"author,omitempty"`
	LikeCount   int64             `json:"like_count"`
	PublishedAt *time.Time        `json:"published_at"`
	CreatedAt   time.Time         `json:"created_at"`
	UpdatedAt   time.Time         `json:"updated_at"`
}

type AdminCategoryRef struct {
	ID   uint   `json:"id"`
	Name string `json:"name"`
}

type AdminAuthorRef struct {
	ID       uint   `json:"id"`
	Username string `json:"username"`
}

type ListAdminResult struct {
	List     []AdminArticleSummary `json:"list"`
	Page     int                   `json:"page"`
	PageSize int                   `json:"page_size"`
	Total    int64                 `json:"total"`
}

type ArticleStatusStats struct {
	Total     int64 `json:"total"`
	Published int64 `json:"published"`
	Draft     int64 `json:"draft"`
}

type ArticleEditorDetail struct {
	ID          uint       `json:"id"`
	Title       string     `json:"title"`
	Slug        string     `json:"slug"`
	Content     string     `json:"content"`
	CoverImage  string     `json:"cover_image"`
	CategoryID  *uint      `json:"category_id"`
	Status      string     `json:"status"`
	IsPinned    bool       `json:"is_pinned"`
	PublishedAt *time.Time `json:"published_at"`
	CreatedAt   time.Time  `json:"created_at"`
	UpdatedAt   time.Time  `json:"updated_at"`
}

type CreateArticleInput struct {
	Title       string     `json:"title"`
	Content     string     `json:"content"`
	CoverImage  string     `json:"cover_image"`
	CategoryID  *uint      `json:"category_id"`
	Status      string     `json:"status"`
	IsPinned    bool       `json:"is_pinned"`
	PublishedAt *time.Time `json:"published_at"`
	AuthorID    uint       `json:"author_id"`
}

type UpdateArticleInput struct {
	Title            *string    `json:"title"`
	Content          *string    `json:"content"`
	CoverImage       *string    `json:"cover_image"`
	CategoryID       *uint      `json:"category_id"`
	ClearCategory    bool       `json:"-"`
	Status           *string    `json:"status"`
	IsPinned         *bool      `json:"is_pinned"`
	PublishedAt      *time.Time `json:"published_at"`
	ClearPublishTime bool       `json:"-"`
}

type UpdateStatusInput struct {
	Status      string
	PublishedAt *time.Time
}

type ArticleService struct {
	db       *gorm.DB
	renderer *RendererService
}

func NewArticleService(db *gorm.DB, renderer *RendererService) *ArticleService {
	return &ArticleService{db: db, renderer: renderer}
}

func (s *ArticleService) ListPublished(ctx context.Context, input ListPublishedInput) (*ListPublishedResult, error) {
	limit := input.Limit
	if limit <= 0 {
		limit = 12
	}
	if limit > 50 {
		limit = 50
	}
	cursor, err := DecodeCursor(input.Cursor)
	if err != nil {
		return nil, err
	}

	query := s.db.WithContext(ctx).
		Model(&model.Article{}).
		Preload("Category").
		Preload("Author").
		Where("articles.status = ? AND articles.published_at IS NOT NULL AND articles.published_at <= ?", "published", time.Now().UTC())

	if input.CategorySlug != "" {
		query = query.Joins("LEFT JOIN categories ON categories.id = articles.category_id").Where("categories.slug = ?", input.CategorySlug)
	}
	if keyword := strings.TrimSpace(input.Keyword); keyword != "" {
		like := "%" + keyword + "%"
		query = query.Where("(articles.title LIKE ? OR articles.excerpt LIKE ?)", like, like)
	}
	if cursor != nil {
		query = query.Where(
			"(CASE WHEN articles.is_pinned THEN 1 ELSE 0 END < ?) OR "+
				"((CASE WHEN articles.is_pinned THEN 1 ELSE 0 END = ?) AND articles.published_at < ?) OR "+
				"((CASE WHEN articles.is_pinned THEN 1 ELSE 0 END = ?) AND articles.published_at = ? AND articles.id < ?)",
			cursor.IsPinned, cursor.IsPinned, cursor.PublishedAt,
			cursor.IsPinned, cursor.PublishedAt, cursor.ID,
		)
	}

	var articles []model.Article
	if err := query.Order("articles.is_pinned DESC").Order("articles.published_at DESC").Order("articles.id DESC").Limit(limit + 1).Find(&articles).Error; err != nil {
		return nil, err
	}

	hasMore := len(articles) > limit
	if hasMore {
		articles = articles[:limit]
	}

	likeMap, err := s.likeCounts(ctx, articles)
	if err != nil {
		return nil, err
	}
	list := make([]PublicArticleSummary, 0, len(articles))
	for _, article := range articles {
		list = append(list, toPublicSummary(article, likeMap[article.ID]))
	}

	nextCursor := ""
	if hasMore && len(articles) > 0 {
		last := articles[len(articles)-1]
		cursorValue, err := EncodeCursor(Cursor{
			IsPinned:    boolToInt(last.IsPinned),
			PublishedAt: derefTime(last.PublishedAt),
			ID:          last.ID,
		})
		if err != nil {
			return nil, err
		}
		nextCursor = cursorValue
	}

	return &ListPublishedResult{
		List:       list,
		NextCursor: nextCursor,
		HasMore:    hasMore,
	}, nil
}

func (s *ArticleService) GetPublishedBySlug(ctx context.Context, slug string, anonymousID string) (*PublicArticleDetail, string, error) {
	if !IsValidSlug(slug) {
		return nil, "", NewAppError(404, "not_found", "文章不存在")
	}

	article, redirectSlug, err := s.findVisibleArticleBySlug(ctx, slug)
	if err != nil {
		return nil, "", err
	}
	if redirectSlug != "" {
		return nil, redirectSlug, nil
	}

	contentHTML, excerpt, err := s.renderer.RenderSafeHTML(article.Content)
	if err != nil {
		return nil, "", err
	}
	likeMap, err := s.likeCounts(ctx, []model.Article{*article})
	if err != nil {
		return nil, "", err
	}

	userLiked := false
	userBookmarked := false
	authorFollowed := false
	if anonymousID != "" {
		var count int64
		if err := s.db.WithContext(ctx).Model(&model.Like{}).
			Where("article_id = ? AND anonymous_id = ?", article.ID, anonymousID).
			Count(&count).Error; err != nil {
			return nil, "", err
		}
		userLiked = count > 0

		if err := s.db.WithContext(ctx).Model(&model.Bookmark{}).
			Where("article_id = ? AND anonymous_id = ?", article.ID, anonymousID).
			Count(&count).Error; err != nil {
			return nil, "", err
		}
		userBookmarked = count > 0

		if err := s.db.WithContext(ctx).Model(&model.AuthorFollow{}).
			Where("author_id = ? AND anonymous_id = ?", article.AuthorID, anonymousID).
			Count(&count).Error; err != nil {
			return nil, "", err
		}
		authorFollowed = count > 0
	}

	return &PublicArticleDetail{
		ID:             article.ID,
		Title:          article.Title,
		Slug:           article.Slug,
		ContentHTML:    contentHTML,
		Excerpt:        excerpt,
		CoverImage:     article.CoverImage,
		Category:       toPublicCategory(article.Category),
		Author:         toPublicAuthor(article.Author),
		IsPinned:       article.IsPinned,
		LikeCount:      likeMap[article.ID],
		UserLiked:      userLiked,
		UserBookmarked: userBookmarked,
		AuthorFollowed: authorFollowed,
		ReadTimeMin:    estimateReadTimeMin(article.Content),
		PublishedAt:    article.PublishedAt,
		CreatedAt:      article.CreatedAt,
		UpdatedAt:      article.UpdatedAt,
	}, "", nil
}

func (s *ArticleService) ListRelated(ctx context.Context, articleID uint, categoryID *uint, limit int) ([]PublicArticleSummary, error) {
	if limit <= 0 {
		limit = 3
	}
	if limit > 20 {
		limit = 20
	}

	query := s.db.WithContext(ctx).
		Model(&model.Article{}).
		Preload("Category").
		Preload("Author").
		Where("status = ? AND published_at IS NOT NULL AND published_at <= ?", "published", time.Now().UTC()).
		Where("id <> ?", articleID)

	if categoryID != nil {
		query = query.Where("category_id = ?", *categoryID)
	}

	var articles []model.Article
	if err := query.Order("published_at DESC").Order("id DESC").Limit(limit).Find(&articles).Error; err != nil {
		return nil, err
	}

	likeMap, err := s.likeCounts(ctx, articles)
	if err != nil {
		return nil, err
	}
	list := make([]PublicArticleSummary, 0, len(articles))
	for _, article := range articles {
		list = append(list, toPublicSummary(article, likeMap[article.ID]))
	}
	return list, nil
}

func (s *ArticleService) GetByID(ctx context.Context, id uint) (*ArticleEditorDetail, error) {
	var article model.Article
	if err := s.db.WithContext(ctx).First(&article, id).Error; err != nil {
		return nil, NewAppError(404, "not_found", "文章不存在")
	}
	return &ArticleEditorDetail{
		ID:          article.ID,
		Title:       article.Title,
		Slug:        article.Slug,
		Content:     article.Content,
		CoverImage:  article.CoverImage,
		CategoryID:  article.CategoryID,
		Status:      article.Status,
		IsPinned:    article.IsPinned,
		PublishedAt: article.PublishedAt,
		CreatedAt:   article.CreatedAt,
		UpdatedAt:   article.UpdatedAt,
	}, nil
}

func (s *ArticleService) ListAdmin(ctx context.Context, input ListAdminInput) (*ListAdminResult, error) {
	if input.Page <= 0 {
		input.Page = 1
	}
	if input.PageSize <= 0 {
		input.PageSize = 20
	}
	if input.PageSize > 100 {
		input.PageSize = 100
	}
	if input.SortOrder != "asc" {
		input.SortOrder = "desc"
	}

	query := s.db.WithContext(ctx).Model(&model.Article{}).Preload("Category").Preload("Author")
	if input.Status != "" {
		query = query.Where("status = ?", input.Status)
	}
	if input.CategoryID > 0 {
		query = query.Where("category_id = ?", input.CategoryID)
	}
	if keyword := strings.TrimSpace(input.Keyword); keyword != "" {
		query = query.Where("title LIKE ?", "%"+keyword+"%")
	}

	var total int64
	if err := query.Count(&total).Error; err != nil {
		return nil, err
	}

	orderField := "updated_at"
	switch input.SortBy {
	case "published_at", "created_at", "updated_at":
		orderField = input.SortBy
	case "like_count":
		orderField = "like_count"
	}

	var articles []model.Article
	if orderField == "like_count" {
		query = query.
			Select("articles.*").
			Joins("LEFT JOIN (SELECT article_id, COUNT(*) AS like_count FROM likes GROUP BY article_id) AS like_stats ON like_stats.article_id = articles.id").
			Order(fmt.Sprintf("COALESCE(like_stats.like_count, 0) %s", input.SortOrder))
	} else {
		query = query.Order(fmt.Sprintf("%s %s", orderField, input.SortOrder))
	}
	if err := query.Order("id DESC").Offset((input.Page - 1) * input.PageSize).Limit(input.PageSize).Find(&articles).Error; err != nil {
		return nil, err
	}

	likeMap, err := s.likeCounts(ctx, articles)
	if err != nil {
		return nil, err
	}

	list := make([]AdminArticleSummary, 0, len(articles))
	for _, article := range articles {
		item := AdminArticleSummary{
			ID:          article.ID,
			Title:       article.Title,
			Slug:        article.Slug,
			CoverImage:  article.CoverImage,
			Status:      article.Status,
			IsPinned:    article.IsPinned,
			LikeCount:   likeMap[article.ID],
			PublishedAt: article.PublishedAt,
			CreatedAt:   article.CreatedAt,
			UpdatedAt:   article.UpdatedAt,
		}
		if article.Category != nil {
			item.Category = &AdminCategoryRef{
				ID:   article.Category.ID,
				Name: article.Category.Name,
			}
		}
		if article.Author != nil {
			item.Author = &AdminAuthorRef{
				ID:       article.Author.ID,
				Username: article.Author.Username,
			}
		}
		list = append(list, item)
	}

	return &ListAdminResult{
		List:     list,
		Page:     input.Page,
		PageSize: input.PageSize,
		Total:    total,
	}, nil
}

func (s *ArticleService) CountByStatus(ctx context.Context) (*ArticleStatusStats, error) {
	var rows []struct {
		Status string
		Count  int64
	}
	if err := s.db.WithContext(ctx).
		Model(&model.Article{}).
		Select("status, COUNT(*) AS count").
		Group("status").
		Scan(&rows).Error; err != nil {
		return nil, err
	}

	stats := &ArticleStatusStats{}
	for _, row := range rows {
		stats.Total += row.Count
		switch row.Status {
		case "published":
			stats.Published = row.Count
		case "draft":
			stats.Draft = row.Count
		}
	}
	return stats, nil
}

func (s *ArticleService) Create(ctx context.Context, input CreateArticleInput) (*model.Article, error) {
	title := strings.TrimSpace(input.Title)
	if len([]rune(title)) == 0 || len([]rune(title)) > 120 {
		return nil, NewAppError(400, "invalid_params", "文章标题长度需为 1-120 字符")
	}
	if strings.TrimSpace(input.Content) == "" {
		return nil, NewAppError(400, "invalid_params", "文章内容不能为空")
	}
	if len([]rune(input.Content)) > 200000 {
		return nil, NewAppError(400, "invalid_params", "文章内容过长")
	}
	if !ValidateCoverImagePath(input.CoverImage) {
		return nil, NewAppError(400, "invalid_params", "cover_image 只能引用站内上传路径或 https 图片")
	}

	status := input.Status
	if status == "" {
		status = "draft"
	}
	if status != "draft" && status != "published" {
		return nil, NewAppError(400, "invalid_params", "status 必须为 draft 或 published")
	}

	if input.CategoryID != nil {
		var exists int64
		if err := s.db.WithContext(ctx).Model(&model.Category{}).Where("id = ?", *input.CategoryID).Count(&exists).Error; err != nil {
			return nil, err
		}
		if exists == 0 {
			return nil, NewAppError(400, "invalid_params", "分类不存在")
		}
	}

	slug, err := s.nextUniqueSlug(ctx, Slugify(title), 0)
	if err != nil {
		return nil, err
	}
	_, excerpt, err := s.renderer.RenderSafeHTML(input.Content)
	if err != nil {
		return nil, err
	}

	publishedAt := input.PublishedAt
	if status == "published" && publishedAt == nil {
		now := time.Now().UTC()
		publishedAt = &now
	}

	article := &model.Article{
		Title:       title,
		Slug:        slug,
		Content:     input.Content,
		CoverImage:  input.CoverImage,
		Excerpt:     excerpt,
		CategoryID:  input.CategoryID,
		AuthorID:    input.AuthorID,
		Status:      status,
		IsPinned:    input.IsPinned,
		PublishedAt: publishedAt,
	}

	if err := s.db.WithContext(ctx).Create(article).Error; err != nil {
		return nil, err
	}
	return article, nil
}

func (s *ArticleService) Update(ctx context.Context, id uint, input UpdateArticleInput) (*model.Article, error) {
	var article model.Article
	if err := s.db.WithContext(ctx).First(&article, id).Error; err != nil {
		return nil, NewAppError(404, "not_found", "文章不存在")
	}

	oldSlug := article.Slug
	shouldSaveSlugHistory := false
	if input.Title != nil {
		title := strings.TrimSpace(*input.Title)
		if len([]rune(title)) == 0 || len([]rune(title)) > 120 {
			return nil, NewAppError(400, "invalid_params", "文章标题长度需为 1-120 字符")
		}
		article.Title = title
		newSlugBase := Slugify(title)
		if newSlugBase != article.Slug {
			slug, err := s.nextUniqueSlug(ctx, newSlugBase, article.ID)
			if err != nil {
				return nil, err
			}
			if slug != article.Slug {
				article.Slug = slug
				shouldSaveSlugHistory = true
			}
		}
	}
	if input.Content != nil {
		content := *input.Content
		if strings.TrimSpace(content) == "" {
			return nil, NewAppError(400, "invalid_params", "文章内容不能为空")
		}
		if len([]rune(content)) > 200000 {
			return nil, NewAppError(400, "invalid_params", "文章内容过长")
		}
		article.Content = content
		_, excerpt, err := s.renderer.RenderSafeHTML(content)
		if err != nil {
			return nil, err
		}
		article.Excerpt = excerpt
	}
	if input.CoverImage != nil {
		if !ValidateCoverImagePath(*input.CoverImage) {
			return nil, NewAppError(400, "invalid_params", "cover_image 只能引用站内上传路径或 https 图片")
		}
		article.CoverImage = *input.CoverImage
	}
	if input.ClearCategory {
		article.CategoryID = nil
	} else if input.CategoryID != nil {
		var exists int64
		if err := s.db.WithContext(ctx).Model(&model.Category{}).Where("id = ?", *input.CategoryID).Count(&exists).Error; err != nil {
			return nil, err
		}
		if exists == 0 {
			return nil, NewAppError(400, "invalid_params", "分类不存在")
		}
		article.CategoryID = input.CategoryID
	}
	if input.IsPinned != nil {
		article.IsPinned = *input.IsPinned
	}
	if input.Status != nil {
		switch *input.Status {
		case "draft":
			article.Status = "draft"
		case "published":
			article.Status = "published"
			if article.PublishedAt == nil {
				now := time.Now().UTC()
				article.PublishedAt = &now
			}
		default:
			return nil, NewAppError(400, "invalid_params", "status 必须为 draft 或 published")
		}
	}
	if input.ClearPublishTime {
		article.PublishedAt = nil
	} else if input.PublishedAt != nil {
		article.PublishedAt = input.PublishedAt
	}
	if article.Status == "published" && article.PublishedAt == nil {
		now := time.Now().UTC()
		article.PublishedAt = &now
	}

	err := s.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		if shouldSaveSlugHistory {
			if err := tx.Create(&model.SlugHistory{
				ArticleID: &article.ID,
				OldSlug:   oldSlug,
			}).Error; err != nil && !strings.Contains(strings.ToLower(err.Error()), "unique") {
				return err
			}
		}
		return tx.Save(&article).Error
	})
	if err != nil {
		return nil, err
	}
	return &article, nil
}

func (s *ArticleService) UpdateStatus(ctx context.Context, id uint, input UpdateStatusInput) (*model.Article, error) {
	status := input.Status
	if status != "draft" && status != "published" {
		return nil, NewAppError(400, "invalid_params", "status 必须为 draft 或 published")
	}

	var article model.Article
	if err := s.db.WithContext(ctx).First(&article, id).Error; err != nil {
		return nil, NewAppError(404, "not_found", "文章不存在")
	}

	article.Status = status
	if status == "published" {
		if input.PublishedAt != nil {
			article.PublishedAt = input.PublishedAt
		}
		if article.PublishedAt == nil {
			now := time.Now().UTC()
			article.PublishedAt = &now
		}
	}

	if err := s.db.WithContext(ctx).Save(&article).Error; err != nil {
		return nil, err
	}
	return &article, nil
}

func (s *ArticleService) Delete(ctx context.Context, id uint) error {
	var article model.Article
	if err := s.db.WithContext(ctx).First(&article, id).Error; err != nil {
		return NewAppError(404, "not_found", "文章不存在")
	}

	return s.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		var count int64
		if err := tx.Model(&model.SlugHistory{}).Where("old_slug = ?", article.Slug).Count(&count).Error; err != nil {
			return err
		}
		if count == 0 {
			if err := tx.Create(&model.SlugHistory{
				OldSlug: article.Slug,
			}).Error; err != nil {
				return err
			}
		}
		if err := tx.Model(&model.SlugHistory{}).Where("article_id = ?", article.ID).Update("article_id", nil).Error; err != nil {
			return err
		}
		return tx.Delete(&article).Error
	})
}

func (s *ArticleService) PreviewMarkdown(content string) (string, string, error) {
	if len([]rune(content)) > 200000 {
		return "", "", NewAppError(400, "invalid_params", "Markdown 内容过长")
	}
	return s.renderer.RenderSafeHTML(content)
}

func (s *ArticleService) findVisibleArticleBySlug(ctx context.Context, slug string) (*model.Article, string, error) {
	var article model.Article
	err := s.db.WithContext(ctx).
		Preload("Category").
		Preload("Author").
		Where("slug = ? AND status = ? AND published_at IS NOT NULL AND published_at <= ?", slug, "published", time.Now().UTC()).
		First(&article).Error
	if err == nil {
		return &article, "", nil
	}
	if !errors.Is(err, gorm.ErrRecordNotFound) {
		return nil, "", err
	}

	var history model.SlugHistory
	if err := s.db.WithContext(ctx).Where("old_slug = ?", slug).First(&history).Error; err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, "", NewAppError(404, "not_found", "文章不存在")
		}
		return nil, "", err
	}
	if history.ArticleID == nil {
		return nil, "", NewAppError(404, "not_found", "文章不存在")
	}
	if err := s.db.WithContext(ctx).
		Preload("Category").
		Preload("Author").
		Where("id = ? AND status = ? AND published_at IS NOT NULL AND published_at <= ?", *history.ArticleID, "published", time.Now().UTC()).
		First(&article).Error; err != nil {
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return nil, "", NewAppError(404, "not_found", "文章不存在")
		}
		return nil, "", err
	}
	return &article, article.Slug, nil
}

func (s *ArticleService) nextUniqueSlug(ctx context.Context, base string, ignoreArticleID uint) (string, error) {
	base = strings.TrimSpace(strings.ToLower(base))
	if !IsValidSlug(base) {
		base = Slugify(base)
	}
	if !IsValidSlug(base) {
		base = fmt.Sprintf("article-%d", time.Now().Unix())
	}

	candidate := base
	for i := 2; i < 1000; i++ {
		taken, err := s.slugTaken(ctx, candidate, ignoreArticleID)
		if err != nil {
			return "", err
		}
		if !taken {
			return candidate, nil
		}
		candidate = fmt.Sprintf("%s-%d", base, i)
	}
	return "", NewAppError(409, "conflict", "无法生成唯一 slug")
}

func (s *ArticleService) slugTaken(ctx context.Context, slug string, ignoreArticleID uint) (bool, error) {
	var count int64
	query := s.db.WithContext(ctx).Model(&model.Article{}).Where("slug = ?", slug)
	if ignoreArticleID > 0 {
		query = query.Where("id <> ?", ignoreArticleID)
	}
	if err := query.Count(&count).Error; err != nil {
		return false, err
	}
	if count > 0 {
		return true, nil
	}
	if err := s.db.WithContext(ctx).Model(&model.SlugHistory{}).Where("old_slug = ?", slug).Count(&count).Error; err != nil {
		return false, err
	}
	return count > 0, nil
}

func (s *ArticleService) likeCounts(ctx context.Context, articles []model.Article) (map[uint]int64, error) {
	result := make(map[uint]int64, len(articles))
	if len(articles) == 0 {
		return result, nil
	}
	ids := make([]uint, 0, len(articles))
	for _, article := range articles {
		ids = append(ids, article.ID)
	}
	type row struct {
		ArticleID uint
		Count     int64
	}
	var rows []row
	if err := s.db.WithContext(ctx).
		Model(&model.Like{}).
		Select("article_id, COUNT(*) AS count").
		Where("article_id IN ?", ids).
		Group("article_id").
		Scan(&rows).Error; err != nil {
		return nil, err
	}
	for _, row := range rows {
		result[row.ArticleID] = row.Count
	}
	return result, nil
}

func toPublicSummary(article model.Article, likeCount int64) PublicArticleSummary {
	return PublicArticleSummary{
		ID:          article.ID,
		Title:       article.Title,
		Slug:        article.Slug,
		CoverImage:  article.CoverImage,
		Excerpt:     article.Excerpt,
		Category:    toPublicCategory(article.Category),
		Author:      toPublicAuthor(article.Author),
		IsPinned:    article.IsPinned,
		LikeCount:   likeCount,
		ReadTimeMin: estimateReadTimeMin(article.Content),
		PublishedAt: article.PublishedAt,
	}
}

func estimateReadTimeMin(content string) int {
	runes := len([]rune(content))
	if runes <= 0 {
		return 1
	}
	minutes := runes / 400
	if minutes < 1 {
		return 1
	}
	return minutes
}

func toPublicCategory(category *model.Category) *PublicCategory {
	if category == nil {
		return nil
	}
	return &PublicCategory{
		ID:   category.ID,
		Name: category.Name,
		Slug: category.Slug,
	}
}

func toPublicAuthor(author *model.User) *PublicAuthor {
	if author == nil {
		return nil
	}
	return &PublicAuthor{
		ID:       author.ID,
		Username: author.Username,
	}
}

func boolToInt(value bool) int {
	if value {
		return 1
	}
	return 0
}

func derefTime(value *time.Time) time.Time {
	if value == nil {
		return time.Time{}
	}
	return *value
}
