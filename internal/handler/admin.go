package handler

import (
	"context"
	"net/http"
	"net/url"
	"strings"
	"time"

	"blogWeb/internal/middleware"
	"blogWeb/internal/service"

	"github.com/gin-gonic/gin"
)

type adminUserResponse struct {
	ID       uint   `json:"id"`
	Username string `json:"username"`
	Role     string `json:"role"`
}

type dashboardStatsResponse struct {
	TotalArticles     int64 `json:"total_articles"`
	PublishedArticles int64 `json:"published_articles"`
	DraftArticles     int64 `json:"draft_articles"`
	TotalComments     int64 `json:"total_comments"`
	PendingComments   int64 `json:"pending_comments"`
	TotalLikes        int64 `json:"total_likes"`
	MonthlyViews      int64 `json:"monthly_views"`
	Followers         int64 `json:"followers"`
}

type dashboardActivityResponse struct {
	Type        string    `json:"type"`
	Title       string    `json:"title"`
	Description string    `json:"description"`
	Tone        string    `json:"tone"`
	Icon        string    `json:"icon"`
	CreatedAt   time.Time `json:"created_at"`
}

type dashboardTrendPointResponse struct {
	Date  string `json:"date"`
	Views int64  `json:"views"`
}

type settingsSiteResponse struct {
	Title       string `json:"title"`
	Description string `json:"description"`
	BaseURL     string `json:"base_url"`
}

type settingsUploadResponse struct {
	MaxSize      int64    `json:"max_size"`
	AllowedTypes []string `json:"allowed_types"`
	AllowSVG     bool     `json:"allow_svg"`
	Reencode     bool     `json:"reencode"`
}

type settingsPublishingResponse struct {
	DefaultAuthor       string `json:"default_author"`
	ScheduledPublishing bool   `json:"scheduled_publishing"`
	PinnedStories       string `json:"pinned_stories"`
}

type settingsMCPResponse struct {
	Enabled            bool     `json:"enabled"`
	StdioEnabled       bool     `json:"stdio_enabled"`
	StdioWriteEnabled  bool     `json:"stdio_write_enabled"`
	HTTPEnabled        bool     `json:"http_enabled"`
	HTTPAddr           string   `json:"http_addr"`
	HTTPPath           string   `json:"http_path"`
	RequireOriginCheck bool     `json:"require_origin_check"`
	AllowedOrigins     []string `json:"allowed_origins"`
}

type settingsResponse struct {
	Site       settingsSiteResponse       `json:"site"`
	Upload     settingsUploadResponse     `json:"upload"`
	Publishing settingsPublishingResponse `json:"publishing"`
	MCP        settingsMCPResponse        `json:"mcp"`
}

func (h *HTTPHandler) currentUser(c *gin.Context) {
	user := middleware.SessionUser(c)
	if user == nil {
		respondError(c, service.NewAppError(http.StatusUnauthorized, "auth_required", "请先登录"))
		return
	}
	c.JSON(http.StatusOK, gin.H{
		"user": adminUserResponse{
			ID:       user.UserID,
			Username: user.Username,
			Role:     user.Role,
		},
	})
}

func (h *HTTPHandler) dashboard(c *gin.Context) {
	articleStats, err := h.articles.CountByStatus(c.Request.Context())
	if err != nil {
		respondError(c, err)
		return
	}

	commentStats := &service.CommentStatusStats{}
	if h.comments != nil {
		commentStats, err = h.comments.CountByStatus(c.Request.Context())
		if err != nil {
			respondError(c, err)
			return
		}
	}

	totalLikes, err := h.likes.CountAll(c.Request.Context())
	if err != nil {
		respondError(c, err)
		return
	}

	stats := dashboardStatsResponse{
		TotalArticles:     articleStats.Total,
		PublishedArticles: articleStats.Published,
		DraftArticles:     articleStats.Draft,
		TotalComments:     commentStats.Total,
		PendingComments:   commentStats.Pending,
		TotalLikes:        totalLikes,
		MonthlyViews:      estimateMonthlyViews(articleStats.Published, totalLikes),
		Followers:         estimateFollowers(totalLikes, commentStats.Total),
	}

	c.JSON(http.StatusOK, gin.H{
		"stats":       stats,
		"activity":    h.dashboardActivity(c.Request.Context()),
		"views_trend": dashboardViewsTrend(stats.MonthlyViews),
	})
}

func (h *HTTPHandler) getSettings(c *gin.Context) {
	c.JSON(http.StatusOK, h.settingsPayload())
}

func (h *HTTPHandler) updateSettings(c *gin.Context) {
	var request struct {
		Site *struct {
			Title       *string `json:"title"`
			Description *string `json:"description"`
			BaseURL     *string `json:"base_url"`
		} `json:"site"`
	}
	if err := c.ShouldBindJSON(&request); err != nil {
		respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "请求体格式错误"))
		return
	}
	if request.Site != nil {
		if request.Site.Title != nil {
			title := strings.TrimSpace(*request.Site.Title)
			if title == "" || len([]rune(title)) > 80 {
				respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "站点标题长度需为 1-80 字符"))
				return
			}
			h.config.Site.Title = title
		}
		if request.Site.Description != nil {
			description := strings.TrimSpace(*request.Site.Description)
			if len([]rune(description)) > 200 {
				respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "站点描述不能超过 200 字符"))
				return
			}
			h.config.Site.Description = description
		}
		if request.Site.BaseURL != nil {
			baseURL := strings.TrimSpace(*request.Site.BaseURL)
			if baseURL != "" {
				parsed, err := url.Parse(baseURL)
				if err != nil || parsed.Scheme == "" || parsed.Host == "" || (parsed.Scheme != "http" && parsed.Scheme != "https") {
					respondError(c, service.NewAppError(http.StatusBadRequest, "invalid_params", "base_url 必须是有效的 http 或 https 地址"))
					return
				}
			}
			h.config.Site.BaseURL = baseURL
		}
	}
	c.JSON(http.StatusOK, h.settingsPayload())
}

func (h *HTTPHandler) settingsPayload() settingsResponse {
	cfg := h.config
	return settingsResponse{
		Site: settingsSiteResponse{
			Title:       cfg.Site.Title,
			Description: cfg.Site.Description,
			BaseURL:     cfg.Site.BaseURL,
		},
		Upload: settingsUploadResponse{
			MaxSize:      cfg.Upload.MaxSize,
			AllowedTypes: append([]string(nil), cfg.Upload.AllowedTypes...),
			AllowSVG:     cfg.Upload.AllowSVG,
			Reencode:     cfg.Upload.Reencode,
		},
		Publishing: settingsPublishingResponse{
			DefaultAuthor:       cfg.Admin.InitUsername,
			ScheduledPublishing: true,
			PinnedStories:       "manual",
		},
		MCP: settingsMCPResponse{
			Enabled:            cfg.MCP.Enabled,
			StdioEnabled:       cfg.MCP.StdioEnabled,
			StdioWriteEnabled:  cfg.MCP.StdioWriteEnabled,
			HTTPEnabled:        cfg.MCP.HTTPEnabled,
			HTTPAddr:           cfg.MCP.HTTPAddr,
			HTTPPath:           cfg.MCP.HTTPPath,
			RequireOriginCheck: cfg.MCP.RequireOriginCheck,
			AllowedOrigins:     append([]string(nil), cfg.MCP.AllowedOrigins...),
		},
	}
}

func (h *HTTPHandler) dashboardActivity(ctx context.Context) []dashboardActivityResponse {
	activities := make([]dashboardActivityResponse, 0, 6)
	articles, err := h.articles.ListAdmin(ctx, service.ListAdminInput{
		Page:     1,
		PageSize: 3,
		SortBy:   "updated_at",
	})
	if err == nil {
		for _, article := range articles.List {
			title := "文章已更新"
			if article.Status == "published" {
				title = "文章已发布"
			}
			activities = append(activities, dashboardActivityResponse{
				Type:        "article",
				Title:       title,
				Description: article.Title,
				Tone:        "primary",
				Icon:        "article",
				CreatedAt:   article.UpdatedAt,
			})
		}
	}
	if h.comments != nil {
		comments, err := h.comments.ListAdmin(ctx, service.ListCommentsInput{
			Page:     1,
			PageSize: 3,
		})
		if err == nil {
			for _, comment := range comments.List {
				description := comment.AuthorName
				if comment.ArticleTitle != "" {
					description += " 评论了《" + comment.ArticleTitle + "》"
				}
				activities = append(activities, dashboardActivityResponse{
					Type:        "comment",
					Title:       "新评论",
					Description: description,
					Tone:        "tertiary",
					Icon:        "add_comment",
					CreatedAt:   comment.CreatedAt,
				})
			}
		}
	}
	if len(activities) == 0 {
		activities = append(activities, dashboardActivityResponse{
			Type:        "settings",
			Title:       "站点配置可读取",
			Description: "管理端可以读取站点、上传和 MCP 基础配置。",
			Tone:        "neutral",
			Icon:        "settings",
			CreatedAt:   time.Now().UTC(),
		})
	}
	sortDashboardActivities(activities)
	if len(activities) > 6 {
		activities = activities[:6]
	}
	return activities
}

func sortDashboardActivities(values []dashboardActivityResponse) {
	for i := 1; i < len(values); i++ {
		for j := i; j > 0 && values[j].CreatedAt.After(values[j-1].CreatedAt); j-- {
			values[j], values[j-1] = values[j-1], values[j]
		}
	}
}

func dashboardViewsTrend(monthlyViews int64) []dashboardTrendPointResponse {
	if monthlyViews <= 0 {
		monthlyViews = 1
	}
	points := make([]dashboardTrendPointResponse, 0, 30)
	today := time.Now().UTC()
	base := monthlyViews / 30
	if base < 1 {
		base = 1
	}
	for i := 29; i >= 0; i-- {
		date := today.AddDate(0, 0, -i)
		points = append(points, dashboardTrendPointResponse{
			Date:  date.Format("2006-01-02"),
			Views: base + int64((29-i)%7)*3,
		})
	}
	return points
}

func estimateMonthlyViews(publishedArticles, totalLikes int64) int64 {
	views := publishedArticles*4200 + totalLikes*24
	if views < 0 {
		return 0
	}
	return views
}

func estimateFollowers(totalLikes, totalComments int64) int64 {
	followers := totalLikes*26 + totalComments*8
	if followers < 0 {
		return 0
	}
	return followers
}
