package handler

import (
	"encoding/json"
	"errors"
	"html/template"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"time"

	"blogWeb/config"
	"blogWeb/internal/middleware"
	"blogWeb/internal/service"

	"github.com/gin-gonic/gin"
)

type HTTPHandler struct {
	config      *config.Config
	projectRoot string
	auth        *service.AuthService
	categories  *service.CategoryService
	articles    *service.ArticleService
	comments    *service.CommentService
	likes       *service.LikeService
	uploads     *service.UploadService
	sessions    *service.SessionManager
	limiter     *service.RateLimiter
}

func NewHTTPHandler(
	cfg *config.Config,
	auth *service.AuthService,
	categories *service.CategoryService,
	articles *service.ArticleService,
	likes *service.LikeService,
	uploads *service.UploadService,
	sessions *service.SessionManager,
	limiter *service.RateLimiter,
	comments ...*service.CommentService,
) *HTTPHandler {
	_, currentFile, _, ok := runtime.Caller(0)
	projectRoot := "."
	if ok {
		projectRoot = filepath.Clean(filepath.Join(filepath.Dir(currentFile), "..", ".."))
	}
	var commentService *service.CommentService
	if len(comments) > 0 {
		commentService = comments[0]
	}
	return &HTTPHandler{
		config:      cfg,
		projectRoot: projectRoot,
		auth:        auth,
		categories:  categories,
		articles:    articles,
		comments:    commentService,
		likes:       likes,
		uploads:     uploads,
		sessions:    sessions,
		limiter:     limiter,
	}
}

func (h *HTTPHandler) Router() *gin.Engine {
	gin.SetMode(gin.ReleaseMode)
	router := gin.New()
	router.Use(gin.Recovery())
	router.Use(gin.Logger())
	router.Use(middleware.SecurityHeaders())
	router.Use(h.ensureAnonymousID())
	router.SetFuncMap(template.FuncMap{
		"safeHTML": func(value string) template.HTML {
			return template.HTML(value)
		},
		"formatDate": func(t *time.Time) string {
			if t == nil {
				return ""
			}
			return t.Format("2006年1月2日")
		},
		"initials": func(name string) string {
			trimmed := strings.TrimSpace(name)
			if trimmed == "" {
				return "?"
			}
			runes := []rune(trimmed)
			return strings.ToUpper(string(runes[0:1]))
		},
		"categoryName": categoryName,
		"authorName":   authorName,
		"authorAvatar": authorAvatar,
		"authorBio":    authorBio,
	})
	router.LoadHTMLGlob(filepath.Join(h.projectRoot, "templates", "*.html"))

	router.GET("/healthz", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "ok"})
	})
	router.GET("/admin", h.adminSPA)
	router.GET("/admin/*filepath", h.adminSPAAsset)
	router.GET("/", h.homePage)
	router.GET("/articles/:slug", h.articlePage)
	router.GET("/categories/:slug", h.categoryPage)

	api := router.Group("/api")
	{
		api.GET("/articles", h.listPublicArticles)
		api.GET("/articles/:slug", h.getPublicArticle)
		api.POST("/articles/:slug/like", h.likeArticle)
		api.POST("/articles/:slug/bookmark", h.bookmarkArticle)
		api.POST("/articles/:slug/comments", h.createComment)
		api.POST("/authors/:id/follow", h.followAuthor)
		api.POST("/newsletter/subscribe", h.subscribeNewsletter)
		api.POST("/likes/batch", h.batchLikes)
		api.POST("/auth/register/code", h.requestRegistrationCode)
		api.POST("/auth/register", h.registerWithEmail)
	}

	admin := api.Group("/admin")
	{
		admin.POST("/login", h.login)
		admin.POST("/logout", middleware.RequireAuth(h.auth), h.logout)
		admin.GET("/csrf-token", middleware.RequireAuth(h.auth), middleware.RequireAdmin(), h.csrfToken)

		protected := admin.Group("")
		protected.Use(middleware.RequireAuth(h.auth), middleware.RequireAdmin())
		{
			protected.GET("/me", h.currentUser)
			protected.GET("/dashboard", h.dashboard)
			protected.GET("/settings", h.getSettings)
			protected.GET("/articles", h.listAdminArticles)
			protected.GET("/articles/:id", h.getAdminArticle)
			protected.GET("/categories", h.listCategories)
			protected.GET("/comments", h.listComments)

			writes := protected.Group("")
			writes.Use(middleware.RequireCSRF(h.sessions))
			{
				writes.POST("/articles", h.createArticle)
				writes.PUT("/articles/:id", h.updateArticle)
				writes.DELETE("/articles/:id", h.deleteArticle)
				writes.POST("/categories", h.createCategory)
				writes.PUT("/categories/:id", h.updateCategory)
				writes.DELETE("/categories/:id", h.deleteCategory)
				writes.PUT("/categories/sort", h.sortCategories)
				writes.PUT("/comments/:id/status", h.updateCommentStatus)
				writes.DELETE("/comments/:id", h.deleteComment)
				writes.POST("/upload", h.upload)
				writes.PUT("/settings", h.updateSettings)
			}
		}
	}

	router.Static("/uploads", h.config.Upload.Dir)
	router.Static("/assets", filepath.Join(h.projectRoot, "public", "assets"))
	return router
}

func (h *HTTPHandler) ensureAnonymousID() gin.HandlerFunc {
	return func(c *gin.Context) {
		if _, err := c.Cookie("anonymous_id"); err == nil {
			c.Next()
			return
		}
		token, err := service.NewToken(16)
		if err == nil {
			c.SetCookie("anonymous_id", token, 86400*365, "/", "", false, true)
		}
		c.Next()
	}
}

func anonymousID(c *gin.Context) string {
	id := strings.TrimSpace(c.GetHeader("X-Anonymous-Id"))
	if id != "" {
		return id
	}
	id, _ = c.Cookie("anonymous_id")
	return strings.TrimSpace(id)
}

func (h *HTTPHandler) listPublicArticles(c *gin.Context) {
	limit, _ := strconv.Atoi(c.DefaultQuery("limit", "12"))
	result, err := h.articles.ListPublished(c.Request.Context(), service.ListPublishedInput{
		Cursor:       c.Query("cursor"),
		CategorySlug: c.Query("category"),
		Keyword:      c.Query("keyword"),
		Limit:        limit,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, result)
}

func (h *HTTPHandler) homePage(c *gin.Context) {
	keyword := strings.TrimSpace(c.Query("keyword"))
	articles, err := h.articles.ListPublished(c.Request.Context(), service.ListPublishedInput{
		Cursor:  c.Query("cursor"),
		Keyword: keyword,
		Limit:   12,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	categories, err := h.categories.List(c.Request.Context())
	if err != nil {
		respondError(c, err)
		return
	}

	var hero *service.PublicArticleSummary
	rest := articles.List
	if keyword == "" && c.Query("cursor") == "" && len(rest) > 0 {
		first := rest[0]
		hero = &first
		rest = rest[1:]
	}

	c.HTML(http.StatusOK, "index.html", gin.H{
		"site": gin.H{
			"title":       h.config.Site.Title,
			"description": h.config.Site.Description,
		},
		"categories": categories,
		"hero":       hero,
		"articles":   rest,
		"hasMore":    articles.HasMore,
		"nextCursor": articles.NextCursor,
		"keyword":    keyword,
	})
}

func (h *HTTPHandler) adminSPA(c *gin.Context) {
	h.serveAdminIndex(c)
}

func (h *HTTPHandler) adminSPAAsset(c *gin.Context) {
	path := strings.TrimPrefix(c.Param("filepath"), "/")
	if path == "" {
		h.serveAdminIndex(c)
		return
	}
	fullPath := filepath.Join(h.projectRoot, "public", "admin", filepath.FromSlash(path))
	if info, err := os.Stat(fullPath); err == nil && !info.IsDir() {
		c.File(fullPath)
		return
	}
	h.serveAdminIndex(c)
}

func (h *HTTPHandler) serveAdminIndex(c *gin.Context) {
	c.Header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
	c.Header("Pragma", "no-cache")
	c.Header("Expires", "0")
	c.File(filepath.Join(h.projectRoot, "public", "admin", "index.html"))
}

func (h *HTTPHandler) articlePage(c *gin.Context) {
	readerID := anonymousID(c)
	article, redirectSlug, err := h.articles.GetPublishedBySlug(c.Request.Context(), c.Param("slug"), readerID)
	if err != nil {
		respondError(c, err)
		return
	}
	if redirectSlug != "" {
		c.Redirect(http.StatusMovedPermanently, "/articles/"+redirectSlug)
		return
	}
	categories, err := h.categories.List(c.Request.Context())
	if err != nil {
		respondError(c, err)
		return
	}
	var categoryID *uint
	if article.Category != nil {
		id := article.Category.ID
		categoryID = &id
	}
	related, err := h.articles.ListRelated(c.Request.Context(), article.ID, categoryID, 3)
	if err != nil {
		respondError(c, err)
		return
	}
	comments := []service.PublicComment{}
	if h.comments != nil {
		comments, err = h.comments.ListApprovedByArticle(c.Request.Context(), article.ID)
		if err != nil {
			respondError(c, err)
			return
		}
	}
	c.HTML(http.StatusOK, "article.html", gin.H{
		"site": gin.H{
			"title":       h.config.Site.Title,
			"description": h.config.Site.Description,
		},
		"categories": categories,
		"article":    article,
		"related":    related,
		"comments":   comments,
		"highlights": articleHighlightsFor(article),
	})
}

func (h *HTTPHandler) categoryPage(c *gin.Context) {
	slug := c.Param("slug")
	category, err := h.categories.GetBySlug(c.Request.Context(), slug)
	if err != nil {
		respondError(c, err)
		return
	}
	articles, err := h.articles.ListPublished(c.Request.Context(), service.ListPublishedInput{
		CategorySlug: slug,
		Limit:        50,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	categories, err := h.categories.List(c.Request.Context())
	if err != nil {
		respondError(c, err)
		return
	}
	c.HTML(http.StatusOK, "category.html", gin.H{
		"site": gin.H{
			"title":       h.config.Site.Title,
			"description": h.config.Site.Description,
		},
		"categories": categories,
		"category":   category,
		"articles":   articles.List,
	})
}

func (h *HTTPHandler) getPublicArticle(c *gin.Context) {
	readerID := anonymousID(c)
	result, redirectSlug, err := h.articles.GetPublishedBySlug(c.Request.Context(), c.Param("slug"), readerID)
	if err != nil {
		respondError(c, err)
		return
	}
	if redirectSlug != "" {
		c.Redirect(http.StatusMovedPermanently, "/api/articles/"+redirectSlug)
		return
	}
	c.JSON(http.StatusOK, result)
}

func (h *HTTPHandler) likeArticle(c *gin.Context) {
	readerID := anonymousID(c)
	if readerID == "" {
		respondError(c, service.NewAppError(400, "invalid_params", "缺少匿名访客标识"))
		return
	}

	article, _, err := h.articles.GetPublishedBySlug(c.Request.Context(), c.Param("slug"), readerID)
	if err != nil {
		respondError(c, err)
		return
	}

	ip := clientIP(c)
	cfg := h.config.RateLimit
	allowed, _, err := h.limiter.Allow(c.Request.Context(), service.LikeRateKey(ip), cfg.LikeIPMaxRequests, time.Duration(cfg.LikeIPWindowSec)*time.Second)
	if err != nil {
		respondError(c, err)
		return
	}
	if !allowed {
		respondError(c, service.NewAppError(429, "rate_limited", "请求过于频繁，请稍后再试"))
		return
	}
	allowed, _, err = h.limiter.Allow(c.Request.Context(), service.LikeArticleRateKey(ip, article.ID), cfg.LikeArticleMaxActions, time.Duration(cfg.LikeArticleWindowSec)*time.Second)
	if err != nil {
		respondError(c, err)
		return
	}
	if !allowed {
		respondError(c, service.NewAppError(429, "rate_limited", "请求过于频繁，请稍后再试"))
		return
	}

	var request struct {
		Action string `json:"action"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}

	result, err := h.likes.Toggle(c.Request.Context(), article.ID, readerID, ip, c.Request.UserAgent(), request.Action)
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, result)
}

func (h *HTTPHandler) bookmarkArticle(c *gin.Context) {
	readerID := anonymousID(c)
	if readerID == "" {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "缺少匿名访客标识"))
		return
	}

	article, _, err := h.articles.GetPublishedBySlug(c.Request.Context(), c.Param("slug"), readerID)
	if err != nil {
		respondError(c, err)
		return
	}

	var request struct {
		Action string `json:"action"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "请求体格式错误"))
		return
	}
	result, err := h.likes.ToggleBookmark(c.Request.Context(), article.ID, readerID, clientIP(c), c.Request.UserAgent(), request.Action)
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, result)
}

func (h *HTTPHandler) followAuthor(c *gin.Context) {
	readerID := anonymousID(c)
	if readerID == "" {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "缺少匿名访客标识"))
		return
	}
	authorID, err := strconv.ParseUint(c.Param("id"), 10, 64)
	if err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "作者 ID 非法"))
		return
	}

	var request struct {
		Action string `json:"action"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "请求体格式错误"))
		return
	}
	result, err := h.likes.ToggleAuthorFollow(c.Request.Context(), uint(authorID), readerID, clientIP(c), c.Request.UserAgent(), request.Action)
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, result)
}

func (h *HTTPHandler) subscribeNewsletter(c *gin.Context) {
	readerID := anonymousID(c)
	var request struct {
		Email string `json:"email"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "请求体格式错误"))
		return
	}
	result, err := h.likes.SubscribeNewsletter(c.Request.Context(), request.Email, readerID, clientIP(c), c.Request.UserAgent())
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusCreated, result)
}

func (h *HTTPHandler) createComment(c *gin.Context) {
	if h.comments == nil {
		respondError(c, service.NewAppError(http.StatusInternalServerError, "internal_error", "评论服务未初始化"))
		return
	}

	readerID := anonymousID(c)

	article, _, err := h.articles.GetPublishedBySlug(c.Request.Context(), c.Param("slug"), readerID)
	if err != nil {
		respondError(c, err)
		return
	}

	ip := clientIP(c)
	cfg := h.config.RateLimit
	commentIPWindowSec := cfg.CommentIPWindowSec
	if commentIPWindowSec <= 0 {
		commentIPWindowSec = 60
	}
	commentIPMaxRequests := cfg.CommentIPMaxRequests
	if commentIPMaxRequests <= 0 {
		commentIPMaxRequests = 10
	}
	commentArticleWindowSec := cfg.CommentArticleWindowSec
	if commentArticleWindowSec <= 0 {
		commentArticleWindowSec = 600
	}
	commentArticleMaxActions := cfg.CommentArticleMaxActions
	if commentArticleMaxActions <= 0 {
		commentArticleMaxActions = 5
	}

	allowed, _, err := h.limiter.Allow(c.Request.Context(), service.CommentRateKey(ip), commentIPMaxRequests, time.Duration(commentIPWindowSec)*time.Second)
	if err != nil {
		respondError(c, err)
		return
	}
	if !allowed {
		respondError(c, service.NewAppError(http.StatusTooManyRequests, "rate_limited", "请求过于频繁，请稍后再试"))
		return
	}
	allowed, _, err = h.limiter.Allow(c.Request.Context(), service.CommentArticleRateKey(ip, article.ID), commentArticleMaxActions, time.Duration(commentArticleWindowSec)*time.Second)
	if err != nil {
		respondError(c, err)
		return
	}
	if !allowed {
		respondError(c, service.NewAppError(http.StatusTooManyRequests, "rate_limited", "请求过于频繁，请稍后再试"))
		return
	}

	var request struct {
		AuthorName string `json:"author_name"`
		Content    string `json:"content"`
		ParentID   *uint  `json:"parent_id"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "请求体格式错误"))
		return
	}

	comment, err := h.comments.Create(c.Request.Context(), service.CreateCommentInput{
		ArticleID:   article.ID,
		ParentID:    request.ParentID,
		AuthorName:  request.AuthorName,
		Content:     request.Content,
		AnonymousID: readerID,
		IPAddress:   ip,
		UserAgent:   c.Request.UserAgent(),
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusCreated, gin.H{
		"id":        comment.ID,
		"parent_id": comment.ParentID,
		"status":    comment.Status,
		"message":   "评论已发布",
	})
}

func (h *HTTPHandler) batchLikes(c *gin.Context) {
	readerID := anonymousID(c)
	if readerID == "" {
		respondError(c, service.NewAppError(400, "invalid_params", "缺少匿名访客标识"))
		return
	}

	var request struct {
		ArticleSlugs []string `json:"article_slugs"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}
	if len(request.ArticleSlugs) > 100 {
		respondError(c, service.NewAppError(400, "invalid_params", "article_slugs 数量不能超过 100"))
		return
	}

	ip := clientIP(c)
	cfg := h.config.RateLimit
	allowed, _, err := h.limiter.Allow(c.Request.Context(), service.LikeRateKey(ip), cfg.LikeIPMaxRequests, time.Duration(cfg.LikeIPWindowSec)*time.Second)
	if err != nil {
		respondError(c, err)
		return
	}
	if !allowed {
		respondError(c, service.NewAppError(429, "rate_limited", "请求过于频繁，请稍后再试"))
		return
	}

	likedMap, err := h.likes.BatchStatusBySlugs(c.Request.Context(), request.ArticleSlugs, readerID)
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"liked_map": likedMap})
}

func (h *HTTPHandler) login(c *gin.Context) {
	var request struct {
		Username string `json:"username"`
		Password string `json:"password"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}

	result, err := h.auth.Login(c.Request.Context(), clientIP(c), h.config.RateLimit, request.Username, request.Password)
	if err != nil {
		respondError(c, err)
		return
	}

	maxAge := h.config.Session.MaxAge
	c.SetCookie(service.AdminSessionCookieName, result.SessionID, maxAge, "/", "", false, true)
	c.JSON(http.StatusOK, gin.H{
		"user": gin.H{
			"id":       result.User.ID,
			"username": result.User.Username,
			"email":    result.User.Email,
			"role":     result.User.Role,
		},
	})
}

func (h *HTTPHandler) requestRegistrationCode(c *gin.Context) {
	var request struct {
		Email string `json:"email"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}
	result, err := h.auth.RequestRegistrationCode(c.Request.Context(), clientIP(c), h.config.RateLimit, request.Email)
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusCreated, result)
}

func (h *HTTPHandler) registerWithEmail(c *gin.Context) {
	var request struct {
		Email           string `json:"email"`
		Code            string `json:"code"`
		Password        string `json:"password"`
		ConfirmPassword string `json:"confirm_password"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}
	user, err := h.auth.RegisterWithEmail(c.Request.Context(), service.RegisterWithEmailInput{
		Email:           request.Email,
		Code:            request.Code,
		Password:        request.Password,
		ConfirmPassword: request.ConfirmPassword,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusCreated, gin.H{
		"user": gin.H{
			"id":       user.ID,
			"username": user.Username,
			"email":    user.Email,
			"role":     user.Role,
		},
	})
}

func (h *HTTPHandler) logout(c *gin.Context) {
	sessionID, _ := c.Cookie(service.AdminSessionCookieName)
	if sessionID != "" {
		_ = h.auth.Logout(c.Request.Context(), sessionID)
	}
	c.SetCookie(service.AdminSessionCookieName, "", -1, "/", "", false, true)
	c.JSON(http.StatusOK, gin.H{"message": "已退出登录"})
}

func (h *HTTPHandler) csrfToken(c *gin.Context) {
	sessionID, _ := c.Cookie(service.AdminSessionCookieName)
	token, err := h.sessions.CSRFToken(c.Request.Context(), sessionID)
	if err != nil || token == "" {
		respondError(c, service.NewAppError(401, "auth_required", "请先登录"))
		return
	}
	c.JSON(http.StatusOK, gin.H{"csrf_token": token})
}

func (h *HTTPHandler) listAdminArticles(c *gin.Context) {
	page, _ := strconv.Atoi(c.DefaultQuery("page", "1"))
	pageSize, _ := strconv.Atoi(c.DefaultQuery("page_size", "20"))
	categoryID64, _ := strconv.ParseUint(c.Query("category_id"), 10, 64)

	result, err := h.articles.ListAdmin(c.Request.Context(), service.ListAdminInput{
		Page:       page,
		PageSize:   pageSize,
		Status:     c.Query("status"),
		CategoryID: uint(categoryID64),
		Keyword:    c.Query("keyword"),
		SortBy:     c.DefaultQuery("sort_by", "updated_at"),
		SortOrder:  c.DefaultQuery("sort_order", "desc"),
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, result)
}

func (h *HTTPHandler) getAdminArticle(c *gin.Context) {
	id, err := strconv.ParseUint(c.Param("id"), 10, 64)
	if err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "文章 ID 非法"))
		return
	}
	result, err := h.articles.GetByID(c.Request.Context(), uint(id))
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, result)
}

func (h *HTTPHandler) createArticle(c *gin.Context) {
	var request struct {
		Title       string `json:"title"`
		Content     string `json:"content"`
		CoverImage  string `json:"cover_image"`
		CategoryID  *uint  `json:"category_id"`
		Status      string `json:"status"`
		IsPinned    bool   `json:"is_pinned"`
		PublishedAt string `json:"published_at"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}
	var publishedAt *time.Time
	if strings.TrimSpace(request.PublishedAt) != "" {
		parsed, err := time.Parse(time.RFC3339, request.PublishedAt)
		if err != nil {
			respondError(c, service.NewAppError(400, "invalid_params", "published_at 格式必须为 RFC3339"))
			return
		}
		publishedAt = &parsed
	}
	sessionUser := middleware.SessionUser(c)
	result, err := h.articles.Create(c.Request.Context(), service.CreateArticleInput{
		Title:       request.Title,
		Content:     request.Content,
		CoverImage:  request.CoverImage,
		CategoryID:  request.CategoryID,
		Status:      request.Status,
		IsPinned:    request.IsPinned,
		PublishedAt: publishedAt,
		AuthorID:    sessionUser.UserID,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusCreated, gin.H{"id": result.ID, "slug": result.Slug})
}

func (h *HTTPHandler) updateArticle(c *gin.Context) {
	id, err := strconv.ParseUint(c.Param("id"), 10, 64)
	if err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "文章 ID 非法"))
		return
	}

	var raw map[string]json.RawMessage
	if err := c.ShouldBindJSON(&raw); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}

	input := service.UpdateArticleInput{}
	if value, ok := raw["title"]; ok {
		var title string
		if err := json.Unmarshal(value, &title); err != nil {
			respondError(c, service.NewAppError(400, "invalid_params", "title 类型错误"))
			return
		}
		input.Title = &title
	}
	if value, ok := raw["content"]; ok {
		var content string
		if err := json.Unmarshal(value, &content); err != nil {
			respondError(c, service.NewAppError(400, "invalid_params", "content 类型错误"))
			return
		}
		input.Content = &content
	}
	if value, ok := raw["cover_image"]; ok {
		var cover string
		if err := json.Unmarshal(value, &cover); err != nil {
			respondError(c, service.NewAppError(400, "invalid_params", "cover_image 类型错误"))
			return
		}
		input.CoverImage = &cover
	}
	if value, ok := raw["category_id"]; ok {
		if string(value) == "null" {
			input.ClearCategory = true
		} else {
			var categoryID uint
			if err := json.Unmarshal(value, &categoryID); err != nil {
				respondError(c, service.NewAppError(400, "invalid_params", "category_id 类型错误"))
				return
			}
			input.CategoryID = &categoryID
		}
	}
	if value, ok := raw["status"]; ok {
		var status string
		if err := json.Unmarshal(value, &status); err != nil {
			respondError(c, service.NewAppError(400, "invalid_params", "status 类型错误"))
			return
		}
		input.Status = &status
	}
	if value, ok := raw["is_pinned"]; ok {
		var isPinned bool
		if err := json.Unmarshal(value, &isPinned); err != nil {
			respondError(c, service.NewAppError(400, "invalid_params", "is_pinned 类型错误"))
			return
		}
		input.IsPinned = &isPinned
	}
	if value, ok := raw["published_at"]; ok {
		if string(value) == `""` || string(value) == "null" {
			input.ClearPublishTime = true
		} else {
			var publishedAt string
			if err := json.Unmarshal(value, &publishedAt); err != nil {
				respondError(c, service.NewAppError(400, "invalid_params", "published_at 类型错误"))
				return
			}
			parsed, err := time.Parse(time.RFC3339, publishedAt)
			if err != nil {
				respondError(c, service.NewAppError(400, "invalid_params", "published_at 格式必须为 RFC3339"))
				return
			}
			input.PublishedAt = &parsed
		}
	}

	result, err := h.articles.Update(c.Request.Context(), uint(id), input)
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"id": result.ID, "slug": result.Slug})
}

func (h *HTTPHandler) deleteArticle(c *gin.Context) {
	id, err := strconv.ParseUint(c.Param("id"), 10, 64)
	if err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "文章 ID 非法"))
		return
	}
	if err := h.articles.Delete(c.Request.Context(), uint(id)); err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"message": "删除成功"})
}

func (h *HTTPHandler) listCategories(c *gin.Context) {
	list, err := h.categories.List(c.Request.Context())
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"list": list})
}

func (h *HTTPHandler) listComments(c *gin.Context) {
	if h.comments == nil {
		respondError(c, service.NewAppError(http.StatusInternalServerError, "internal_error", "评论服务未初始化"))
		return
	}
	page, _ := strconv.Atoi(c.DefaultQuery("page", "1"))
	pageSize, _ := strconv.Atoi(c.DefaultQuery("page_size", "20"))
	result, err := h.comments.ListAdmin(c.Request.Context(), service.ListCommentsInput{
		Page:     page,
		PageSize: pageSize,
		Status:   c.Query("status"),
		Keyword:  c.Query("keyword"),
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, result)
}

func (h *HTTPHandler) createCategory(c *gin.Context) {
	var request struct {
		Name      string `json:"name"`
		Slug      string `json:"slug"`
		SortOrder int    `json:"sort_order"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}
	result, err := h.categories.Create(c.Request.Context(), service.CreateCategoryInput{
		Name:      request.Name,
		Slug:      request.Slug,
		SortOrder: request.SortOrder,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusCreated, gin.H{"id": result.ID, "name": result.Name, "slug": result.Slug})
}

func (h *HTTPHandler) updateCategory(c *gin.Context) {
	id, err := strconv.ParseUint(c.Param("id"), 10, 64)
	if err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "分类 ID 非法"))
		return
	}
	var request struct {
		Name      *string `json:"name"`
		Slug      *string `json:"slug"`
		SortOrder *int    `json:"sort_order"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}
	result, err := h.categories.Update(c.Request.Context(), uint(id), service.UpdateCategoryInput{
		Name:      request.Name,
		Slug:      request.Slug,
		SortOrder: request.SortOrder,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"id": result.ID, "name": result.Name, "slug": result.Slug, "sort_order": result.SortOrder})
}

func (h *HTTPHandler) deleteCategory(c *gin.Context) {
	id, err := strconv.ParseUint(c.Param("id"), 10, 64)
	if err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "分类 ID 非法"))
		return
	}
	if err := h.categories.Delete(c.Request.Context(), uint(id)); err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"message": "删除成功"})
}

func (h *HTTPHandler) sortCategories(c *gin.Context) {
	var request struct {
		IDs []uint `json:"ids"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "请求体格式错误"))
		return
	}
	if len(request.IDs) == 0 {
		respondError(c, service.NewAppError(400, "invalid_params", "ids 不能为空"))
		return
	}
	if err := h.categories.Sort(c.Request.Context(), request.IDs); err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"message": "排序更新成功"})
}

func (h *HTTPHandler) updateCommentStatus(c *gin.Context) {
	if h.comments == nil {
		respondError(c, service.NewAppError(http.StatusInternalServerError, "internal_error", "评论服务未初始化"))
		return
	}
	id, err := strconv.ParseUint(c.Param("id"), 10, 64)
	if err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "评论 ID 非法"))
		return
	}
	var request struct {
		Status          string `json:"status"`
		RejectionReason string `json:"rejection_reason"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "请求体格式错误"))
		return
	}
	comment, err := h.comments.UpdateStatus(c.Request.Context(), uint(id), service.UpdateCommentStatusInput{
		Status:          request.Status,
		RejectionReason: request.RejectionReason,
	})
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"id": comment.ID, "status": comment.Status})
}

func (h *HTTPHandler) deleteComment(c *gin.Context) {
	if h.comments == nil {
		respondError(c, service.NewAppError(http.StatusInternalServerError, "internal_error", "评论服务未初始化"))
		return
	}
	id, err := strconv.ParseUint(c.Param("id"), 10, 64)
	if err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "评论 ID 非法"))
		return
	}
	if err := h.comments.Delete(c.Request.Context(), uint(id)); err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"message": "删除成功"})
}

func (h *HTTPHandler) upload(c *gin.Context) {
	fileHeader, err := c.FormFile("file")
	if err != nil {
		respondError(c, service.NewAppError(400, "invalid_params", "缺少 file 文件"))
		return
	}
	result, err := h.uploads.StoreMultipart(fileHeader)
	if err != nil {
		respondError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"url": result.URL, "filename": result.Filename})
}

func clientIP(c *gin.Context) string {
	ip := strings.TrimSpace(c.ClientIP())
	if ip == "" {
		ip = "unknown"
	}
	return ip
}

func respondError(c *gin.Context, err error) {
	if err == nil {
		return
	}
	var appErr *service.AppError
	if errors.As(err, &appErr) {
		if appErr.Code == "" {
			appErr.Code = "internal_error"
		}
		if appErr.Message == "" {
			appErr.Message = "服务端内部错误"
		}
		c.AbortWithStatusJSON(appErr.StatusCode, gin.H{
			"code":    codeValue(appErr),
			"message": appErr.Message,
		})
		return
	}
	c.AbortWithStatusJSON(http.StatusInternalServerError, gin.H{
		"code":    500,
		"message": err.Error(),
	})
}

func codeValue(err *service.AppError) any {
	if err.Code != "" {
		return err.Code
	}
	return err.StatusCode
}
