package service

import (
	"context"
	"net/mail"
	"strings"

	"blogWeb/internal/model"

	"gorm.io/gorm"
)

type LikeService struct {
	db *gorm.DB
}

type LikeResult struct {
	Liked     bool  `json:"liked"`
	LikeCount int64 `json:"like_count"`
}

type BookmarkResult struct {
	Bookmarked    bool  `json:"bookmarked"`
	BookmarkCount int64 `json:"bookmark_count"`
}

type FollowResult struct {
	Following     bool  `json:"following"`
	FollowerCount int64 `json:"follower_count"`
}

type SubscribeResult struct {
	Subscribed bool   `json:"subscribed"`
	Email      string `json:"email"`
}

func NewLikeService(db *gorm.DB) *LikeService {
	return &LikeService{db: db}
}

func (s *LikeService) Toggle(ctx context.Context, articleID uint, anonymousID, ip, userAgent, action string) (*LikeResult, error) {
	action = strings.TrimSpace(action)
	if action != "like" && action != "unlike" {
		return nil, NewAppError(400, "invalid_params", "无效的操作，action 必须为 like 或 unlike")
	}
	if strings.TrimSpace(anonymousID) == "" {
		return nil, NewAppError(400, "invalid_params", "缺少 X-Anonymous-Id 请求头")
	}

	var liked bool
	err := s.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		var existing model.Like
		err := tx.Where("article_id = ? AND anonymous_id = ?", articleID, anonymousID).First(&existing).Error
		switch {
		case action == "like" && err == nil:
			return NewAppError(409, "conflict", "已经点过赞了")
		case action == "like" && err == gorm.ErrRecordNotFound:
			if err := tx.Create(&model.Like{
				ArticleID:   articleID,
				AnonymousID: anonymousID,
				IPAddress:   ip,
				UserAgent:   userAgent,
			}).Error; err != nil {
				return err
			}
			liked = true
			return nil
		case action == "unlike" && err == gorm.ErrRecordNotFound:
			return NewAppError(409, "conflict", "尚未点赞，无法取消")
		case action == "unlike" && err == nil:
			if err := tx.Delete(&existing).Error; err != nil {
				return err
			}
			liked = false
			return nil
		default:
			return err
		}
	})
	if err != nil {
		return nil, err
	}

	count, err := s.CountByArticleID(ctx, articleID)
	if err != nil {
		return nil, err
	}
	return &LikeResult{
		Liked:     liked,
		LikeCount: count,
	}, nil
}

func (s *LikeService) CountByArticleID(ctx context.Context, articleID uint) (int64, error) {
	var count int64
	err := s.db.WithContext(ctx).Model(&model.Like{}).Where("article_id = ?", articleID).Count(&count).Error
	return count, err
}

func (s *LikeService) CountAll(ctx context.Context) (int64, error) {
	var count int64
	err := s.db.WithContext(ctx).Model(&model.Like{}).Count(&count).Error
	return count, err
}

func (s *LikeService) ToggleBookmark(ctx context.Context, articleID uint, anonymousID, ip, userAgent, action string) (*BookmarkResult, error) {
	action = strings.TrimSpace(action)
	if action != "bookmark" && action != "unbookmark" {
		return nil, NewAppError(400, "invalid_params", "无效的操作，action 必须为 bookmark 或 unbookmark")
	}
	if strings.TrimSpace(anonymousID) == "" {
		return nil, NewAppError(400, "invalid_params", "缺少 X-Anonymous-Id 请求头")
	}

	var bookmarked bool
	err := s.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		var existing model.Bookmark
		err := tx.Where("article_id = ? AND anonymous_id = ?", articleID, anonymousID).First(&existing).Error
		switch {
		case action == "bookmark" && err == nil:
			bookmarked = true
			return nil
		case action == "bookmark" && err == gorm.ErrRecordNotFound:
			if err := tx.Create(&model.Bookmark{
				ArticleID:   articleID,
				AnonymousID: anonymousID,
				IPAddress:   truncateString(strings.TrimSpace(ip), 64),
				UserAgent:   truncateString(strings.TrimSpace(userAgent), 255),
			}).Error; err != nil {
				return err
			}
			bookmarked = true
			return nil
		case action == "unbookmark" && err == gorm.ErrRecordNotFound:
			bookmarked = false
			return nil
		case action == "unbookmark" && err == nil:
			if err := tx.Delete(&existing).Error; err != nil {
				return err
			}
			bookmarked = false
			return nil
		default:
			return err
		}
	})
	if err != nil {
		return nil, err
	}

	count, err := s.CountBookmarksByArticleID(ctx, articleID)
	if err != nil {
		return nil, err
	}
	return &BookmarkResult{Bookmarked: bookmarked, BookmarkCount: count}, nil
}

func (s *LikeService) CountBookmarksByArticleID(ctx context.Context, articleID uint) (int64, error) {
	var count int64
	err := s.db.WithContext(ctx).Model(&model.Bookmark{}).Where("article_id = ?", articleID).Count(&count).Error
	return count, err
}

func (s *LikeService) IsBookmarked(ctx context.Context, articleID uint, anonymousID string) (bool, error) {
	if strings.TrimSpace(anonymousID) == "" {
		return false, nil
	}
	var count int64
	err := s.db.WithContext(ctx).Model(&model.Bookmark{}).Where("article_id = ? AND anonymous_id = ?", articleID, anonymousID).Count(&count).Error
	return count > 0, err
}

func (s *LikeService) ToggleAuthorFollow(ctx context.Context, authorID uint, anonymousID, ip, userAgent, action string) (*FollowResult, error) {
	action = strings.TrimSpace(action)
	if action != "follow" && action != "unfollow" {
		return nil, NewAppError(400, "invalid_params", "无效的操作，action 必须为 follow 或 unfollow")
	}
	if strings.TrimSpace(anonymousID) == "" {
		return nil, NewAppError(400, "invalid_params", "缺少 X-Anonymous-Id 请求头")
	}

	var following bool
	err := s.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		var existing model.AuthorFollow
		err := tx.Where("author_id = ? AND anonymous_id = ?", authorID, anonymousID).First(&existing).Error
		switch {
		case action == "follow" && err == nil:
			following = true
			return nil
		case action == "follow" && err == gorm.ErrRecordNotFound:
			var count int64
			if err := tx.Model(&model.User{}).Where("id = ?", authorID).Count(&count).Error; err != nil {
				return err
			}
			if count == 0 {
				return NewAppError(404, "not_found", "作者不存在")
			}
			if err := tx.Create(&model.AuthorFollow{
				AuthorID:    authorID,
				AnonymousID: anonymousID,
				IPAddress:   truncateString(strings.TrimSpace(ip), 64),
				UserAgent:   truncateString(strings.TrimSpace(userAgent), 255),
			}).Error; err != nil {
				return err
			}
			following = true
			return nil
		case action == "unfollow" && err == gorm.ErrRecordNotFound:
			following = false
			return nil
		case action == "unfollow" && err == nil:
			if err := tx.Delete(&existing).Error; err != nil {
				return err
			}
			following = false
			return nil
		default:
			return err
		}
	})
	if err != nil {
		return nil, err
	}

	count, err := s.CountFollowersByAuthorID(ctx, authorID)
	if err != nil {
		return nil, err
	}
	return &FollowResult{Following: following, FollowerCount: count}, nil
}

func (s *LikeService) CountFollowersByAuthorID(ctx context.Context, authorID uint) (int64, error) {
	var count int64
	err := s.db.WithContext(ctx).Model(&model.AuthorFollow{}).Where("author_id = ?", authorID).Count(&count).Error
	return count, err
}

func (s *LikeService) IsFollowingAuthor(ctx context.Context, authorID uint, anonymousID string) (bool, error) {
	if strings.TrimSpace(anonymousID) == "" {
		return false, nil
	}
	var count int64
	err := s.db.WithContext(ctx).Model(&model.AuthorFollow{}).Where("author_id = ? AND anonymous_id = ?", authorID, anonymousID).Count(&count).Error
	return count > 0, err
}

func (s *LikeService) SubscribeNewsletter(ctx context.Context, email, anonymousID, ip, userAgent string) (*SubscribeResult, error) {
	email = strings.ToLower(strings.TrimSpace(email))
	if email == "" {
		return nil, NewAppError(400, "invalid_params", "邮箱不能为空")
	}
	if _, err := mail.ParseAddress(email); err != nil {
		return nil, NewAppError(400, "invalid_params", "邮箱格式不正确")
	}
	if len([]rune(email)) > 255 {
		return nil, NewAppError(400, "invalid_params", "邮箱不能超过 255 个字符")
	}

	subscription := model.NewsletterSubscription{
		Email:       email,
		AnonymousID: strings.TrimSpace(anonymousID),
		Status:      "subscribed",
		IPAddress:   truncateString(strings.TrimSpace(ip), 64),
		UserAgent:   truncateString(strings.TrimSpace(userAgent), 255),
	}
	if err := s.db.WithContext(ctx).
		Where("email = ?", email).
		Assign(map[string]any{
			"anonymous_id": subscription.AnonymousID,
			"status":       subscription.Status,
			"ip_address":   subscription.IPAddress,
			"user_agent":   subscription.UserAgent,
		}).
		FirstOrCreate(&subscription).Error; err != nil {
		return nil, err
	}
	return &SubscribeResult{Subscribed: true, Email: email}, nil
}

func (s *LikeService) BatchStatusBySlugs(ctx context.Context, slugs []string, anonymousID string) (map[string]bool, error) {
	result := make(map[string]bool)
	if len(slugs) == 0 {
		return result, nil
	}

	type row struct {
		Slug string
	}
	var rows []row
	if err := s.db.WithContext(ctx).
		Table("likes").
		Select("articles.slug").
		Joins("JOIN articles ON articles.id = likes.article_id").
		Where("likes.anonymous_id = ? AND articles.slug IN ?", anonymousID, slugs).
		Scan(&rows).Error; err != nil {
		return nil, err
	}
	for _, row := range rows {
		result[row.Slug] = true
	}
	return result, nil
}
