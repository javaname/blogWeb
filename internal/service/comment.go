package service

import (
	"context"
	"net/http"
	"strconv"
	"strings"
	"time"
	"unicode"

	"blogWeb/internal/model"

	"gorm.io/gorm"
)

const (
	CommentStatusApproved = "approved"
	CommentStatusPending  = "pending"
	CommentStatusRejected = "rejected"

	maxCommentAuthorRunes  = 40
	maxCommentContentRunes = 500
)

type CommentService struct {
	db *gorm.DB
}

type CreateCommentInput struct {
	ArticleID   uint
	ParentID    *uint
	AuthorName  string
	Content     string
	AnonymousID string
	IPAddress   string
	UserAgent   string
}

type ListCommentsInput struct {
	Page     int
	PageSize int
	Status   string
	Keyword  string
}

type UpdateCommentStatusInput struct {
	Status          string
	RejectionReason string
}

type PublicComment struct {
	ID           uint            `json:"id"`
	ParentID     *uint           `json:"parent_id,omitempty"`
	AuthorName   string          `json:"author_name"`
	Content      string          `json:"content"`
	RelativeTime string          `json:"relative_time"`
	CreatedAt    time.Time       `json:"created_at"`
	Replies      []PublicComment `json:"replies,omitempty"`
}

type AdminComment struct {
	ID              uint      `json:"id"`
	ArticleID       uint      `json:"article_id"`
	ParentID        *uint     `json:"parent_id,omitempty"`
	ArticleTitle    string    `json:"article_title"`
	AuthorName      string    `json:"author_name"`
	Content         string    `json:"content"`
	Status          string    `json:"status"`
	RejectionReason string    `json:"rejection_reason"`
	CreatedAt       time.Time `json:"created_at"`
	UpdatedAt       time.Time `json:"updated_at"`
}

type CommentListResult struct {
	List     []AdminComment `json:"list"`
	Page     int            `json:"page"`
	PageSize int            `json:"page_size"`
	Total    int64          `json:"total"`
}

type CommentStatusStats struct {
	Total    int64 `json:"total"`
	Approved int64 `json:"approved"`
	Pending  int64 `json:"pending"`
	Rejected int64 `json:"rejected"`
}

type commentPolicyRule struct {
	category string
	message  string
	keywords []string
}

var commentPolicyRules = []commentPolicyRule{
	{
		category: "politics",
		message:  "评论包含政治相关敏感内容，请修改后再提交",
		keywords: []string{
			"政治", "政府", "选举", "总统", "国家主席", "政党", "议会", "国会", "外交", "制裁",
			"游行", "抗议", "革命", "分裂", "独立运动", "台独", "港独", "藏独", "疆独",
			"共产党", "国民党", "习近平", "拜登", "特朗普", "普京",
			"politics", "government", "election", "president", "congress", "parliament", "revolution", "protest",
		},
	},
	{
		category: "violence",
		message:  "评论包含暴力相关敏感内容，请修改后再提交",
		keywords: []string{
			"暴力", "杀人", "杀害", "砍人", "枪击", "枪支", "炸弹", "爆炸", "袭击", "恐怖袭击",
			"暗杀", "绑架", "虐待", "伤害", "打死", "打砸", "纵火", "武器",
			"violence", "kill", "murder", "gun", "bomb", "explosion", "attack", "weapon",
		},
	},
	{
		category: "gore",
		message:  "评论包含血腥相关敏感内容，请修改后再提交",
		keywords: []string{
			"血腥", "鲜血", "流血", "尸体", "尸块", "肢解", "断肢", "内脏", "屠杀", "惨死", "血肉模糊",
			"gore", "blood", "corpse", "dismember", "slaughter",
		},
	},
}

func NewCommentService(db *gorm.DB) *CommentService {
	return &CommentService{db: db}
}

func (s *CommentService) Create(ctx context.Context, input CreateCommentInput) (*model.Comment, error) {
	authorName := strings.TrimSpace(input.AuthorName)
	if authorName == "" {
		authorName = "匿名读者"
	}
	if countRunes(authorName) > maxCommentAuthorRunes {
		return nil, NewAppError(http.StatusBadRequest, "invalid_params", "昵称不能超过 40 个字符")
	}
	if violation := CheckCommentPolicy(authorName); violation != nil {
		return nil, NewAppError(http.StatusBadRequest, "comment_policy_violation", violation.Message)
	}

	content := strings.TrimSpace(input.Content)
	if content == "" {
		return nil, NewAppError(http.StatusBadRequest, "invalid_params", "评论内容不能为空")
	}
	if countRunes(content) > maxCommentContentRunes {
		return nil, NewAppError(http.StatusBadRequest, "invalid_params", "评论内容不能超过 500 个字符")
	}
	if violation := CheckCommentPolicy(content); violation != nil {
		return nil, NewAppError(http.StatusBadRequest, "comment_policy_violation", violation.Message)
	}

	var article model.Article
	if err := s.db.WithContext(ctx).Where("id = ? AND status = ?", input.ArticleID, "published").First(&article).Error; err != nil {
		if err == gorm.ErrRecordNotFound {
			return nil, NewAppError(http.StatusNotFound, "not_found", "文章不存在或尚未发布")
		}
		return nil, err
	}
	if input.ParentID != nil {
		var parent model.Comment
		if err := s.db.WithContext(ctx).
			Where("id = ? AND article_id = ? AND status = ?", *input.ParentID, input.ArticleID, CommentStatusApproved).
			First(&parent).Error; err != nil {
			if err == gorm.ErrRecordNotFound {
				return nil, NewAppError(http.StatusBadRequest, "invalid_params", "回复的评论不存在")
			}
			return nil, err
		}
		if parent.ParentID != nil {
			return nil, NewAppError(http.StatusBadRequest, "invalid_params", "暂不支持多级回复")
		}
	}

	comment := &model.Comment{
		ArticleID:   input.ArticleID,
		ParentID:    input.ParentID,
		AuthorName:  authorName,
		Content:     content,
		Status:      CommentStatusApproved,
		AnonymousID: strings.TrimSpace(input.AnonymousID),
		IPAddress:   truncateString(strings.TrimSpace(input.IPAddress), 64),
		UserAgent:   truncateString(strings.TrimSpace(input.UserAgent), 255),
	}
	if err := s.db.WithContext(ctx).Create(comment).Error; err != nil {
		return nil, err
	}
	return comment, nil
}

func (s *CommentService) ListApprovedByArticle(ctx context.Context, articleID uint) ([]PublicComment, error) {
	var comments []model.Comment
	if err := s.db.WithContext(ctx).
		Where("article_id = ? AND status = ?", articleID, CommentStatusApproved).
		Order("created_at ASC").
		Find(&comments).Error; err != nil {
		return nil, err
	}
	parents := make([]PublicComment, 0, len(comments))
	parentIndex := make(map[uint]int, len(comments))
	for _, comment := range comments {
		publicComment := toPublicComment(comment)
		if comment.ParentID == nil {
			parentIndex[comment.ID] = len(parents)
			parents = append(parents, publicComment)
			continue
		}
		if index, ok := parentIndex[*comment.ParentID]; ok {
			parents[index].Replies = append(parents[index].Replies, publicComment)
		}
	}
	for left, right := 0, len(parents)-1; left < right; left, right = left+1, right-1 {
		parents[left], parents[right] = parents[right], parents[left]
	}
	return parents, nil
}

func (s *CommentService) ListAdmin(ctx context.Context, input ListCommentsInput) (*CommentListResult, error) {
	page := input.Page
	if page < 1 {
		page = 1
	}
	pageSize := input.PageSize
	if pageSize <= 0 || pageSize > 100 {
		pageSize = 20
	}

	query := s.db.WithContext(ctx).Model(&model.Comment{}).Preload("Article")
	if input.Status != "" {
		if !isValidCommentStatus(input.Status) {
			return nil, NewAppError(http.StatusBadRequest, "invalid_params", "评论状态非法")
		}
		query = query.Where("status = ?", input.Status)
	}
	if keyword := strings.TrimSpace(input.Keyword); keyword != "" {
		like := "%" + keyword + "%"
		query = query.Where("author_name LIKE ? OR content LIKE ?", like, like)
	}

	var total int64
	if err := query.Count(&total).Error; err != nil {
		return nil, err
	}

	var comments []model.Comment
	if err := query.
		Order("created_at DESC").
		Limit(pageSize).
		Offset((page - 1) * pageSize).
		Find(&comments).Error; err != nil {
		return nil, err
	}

	result := make([]AdminComment, 0, len(comments))
	for _, comment := range comments {
		result = append(result, toAdminComment(comment))
	}
	return &CommentListResult{List: result, Page: page, PageSize: pageSize, Total: total}, nil
}

func (s *CommentService) CountByStatus(ctx context.Context) (*CommentStatusStats, error) {
	var rows []struct {
		Status string
		Count  int64
	}
	if err := s.db.WithContext(ctx).
		Model(&model.Comment{}).
		Select("status, COUNT(*) AS count").
		Group("status").
		Scan(&rows).Error; err != nil {
		return nil, err
	}

	stats := &CommentStatusStats{}
	for _, row := range rows {
		stats.Total += row.Count
		switch row.Status {
		case CommentStatusApproved:
			stats.Approved = row.Count
		case CommentStatusPending:
			stats.Pending = row.Count
		case CommentStatusRejected:
			stats.Rejected = row.Count
		}
	}
	return stats, nil
}

func (s *CommentService) UpdateStatus(ctx context.Context, id uint, input UpdateCommentStatusInput) (*model.Comment, error) {
	if !isValidCommentStatus(input.Status) {
		return nil, NewAppError(http.StatusBadRequest, "invalid_params", "评论状态非法")
	}
	reason := strings.TrimSpace(input.RejectionReason)
	if input.Status == CommentStatusRejected && reason == "" {
		reason = "不符合评论规范"
	}
	if countRunes(reason) > 200 {
		return nil, NewAppError(http.StatusBadRequest, "invalid_params", "拒绝原因不能超过 200 个字符")
	}

	var comment model.Comment
	if err := s.db.WithContext(ctx).First(&comment, id).Error; err != nil {
		if err == gorm.ErrRecordNotFound {
			return nil, NewAppError(http.StatusNotFound, "not_found", "评论不存在")
		}
		return nil, err
	}

	comment.Status = input.Status
	comment.RejectionReason = reason
	if input.Status != CommentStatusRejected {
		comment.RejectionReason = ""
	}
	if input.Status == CommentStatusApproved {
		if violation := CheckCommentPolicy(comment.AuthorName + " " + comment.Content); violation != nil {
			return nil, NewAppError(http.StatusBadRequest, "comment_policy_violation", violation.Message)
		}
	}
	if err := s.db.WithContext(ctx).Save(&comment).Error; err != nil {
		return nil, err
	}
	return &comment, nil
}

func (s *CommentService) Delete(ctx context.Context, id uint) error {
	result := s.db.WithContext(ctx).Delete(&model.Comment{}, id)
	if result.Error != nil {
		return result.Error
	}
	if result.RowsAffected == 0 {
		return NewAppError(http.StatusNotFound, "not_found", "评论不存在")
	}
	return nil
}

type CommentPolicyViolation struct {
	Category string
	Message  string
}

func CheckCommentPolicy(content string) *CommentPolicyViolation {
	normalized := normalizePolicyText(content)
	if normalized == "" {
		return nil
	}
	for _, rule := range commentPolicyRules {
		for _, keyword := range rule.keywords {
			if strings.Contains(normalized, normalizePolicyText(keyword)) {
				return &CommentPolicyViolation{Category: rule.category, Message: rule.message}
			}
		}
	}
	return nil
}

func toPublicComment(comment model.Comment) PublicComment {
	return PublicComment{
		ID:           comment.ID,
		ParentID:     comment.ParentID,
		AuthorName:   comment.AuthorName,
		Content:      comment.Content,
		RelativeTime: relativeTime(comment.CreatedAt),
		CreatedAt:    comment.CreatedAt,
	}
}

func toAdminComment(comment model.Comment) AdminComment {
	articleTitle := ""
	if comment.Article != nil {
		articleTitle = comment.Article.Title
	}
	return AdminComment{
		ID:              comment.ID,
		ArticleID:       comment.ArticleID,
		ParentID:        comment.ParentID,
		ArticleTitle:    articleTitle,
		AuthorName:      comment.AuthorName,
		Content:         comment.Content,
		Status:          comment.Status,
		RejectionReason: comment.RejectionReason,
		CreatedAt:       comment.CreatedAt,
		UpdatedAt:       comment.UpdatedAt,
	}
}

func isValidCommentStatus(status string) bool {
	switch status {
	case CommentStatusApproved, CommentStatusPending, CommentStatusRejected:
		return true
	default:
		return false
	}
}

func normalizePolicyText(value string) string {
	value = strings.ToLower(strings.TrimSpace(value))
	var builder strings.Builder
	for _, r := range value {
		if unicode.IsLetter(r) || unicode.IsNumber(r) {
			builder.WriteRune(r)
		}
	}
	return builder.String()
}

func relativeTime(value time.Time) string {
	if value.IsZero() {
		return ""
	}
	duration := time.Since(value)
	if duration < time.Minute {
		return "刚刚"
	}
	if duration < time.Hour {
		return strconv.Itoa(int(duration/time.Minute)) + " 分钟前"
	}
	if duration < 24*time.Hour {
		return strconv.Itoa(int(duration/time.Hour)) + " 小时前"
	}
	if duration < 48*time.Hour {
		return "昨天"
	}
	if duration < 30*24*time.Hour {
		return strconv.Itoa(int(duration/(24*time.Hour))) + " 天前"
	}
	return value.Format("2006年1月2日")
}

func countRunes(value string) int {
	return len([]rune(value))
}

func truncateString(value string, maxRunes int) string {
	runes := []rune(value)
	if len(runes) <= maxRunes {
		return value
	}
	return string(runes[:maxRunes])
}
