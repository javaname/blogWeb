package model

import "time"

type Article struct {
	ID          uint       `gorm:"primaryKey" json:"id"`
	Title       string     `gorm:"size:120;not null" json:"title"`
	Slug        string     `gorm:"uniqueIndex;size:160;not null" json:"slug"`
	Content     string     `gorm:"type:text;not null" json:"content"`
	CoverImage  string     `gorm:"size:255;not null;default:''" json:"cover_image"`
	Excerpt     string     `gorm:"type:text;not null;default:''" json:"excerpt"`
	CategoryID  *uint      `json:"category_id"`
	Category    *Category  `json:"category,omitempty"`
	AuthorID    uint       `gorm:"not null" json:"author_id"`
	Author      *User      `json:"author,omitempty"`
	Status      string     `gorm:"size:20;not null;default:draft" json:"status"`
	IsPinned    bool       `gorm:"not null;default:false" json:"is_pinned"`
	PublishedAt *time.Time `json:"published_at"`
	CreatedAt   time.Time  `json:"created_at"`
	UpdatedAt   time.Time  `json:"updated_at"`
}

type Like struct {
	ID          uint      `gorm:"primaryKey" json:"id"`
	ArticleID   uint      `gorm:"uniqueIndex:uq_likes_article_anonymous,priority:1;index;not null" json:"article_id"`
	AnonymousID string    `gorm:"uniqueIndex:uq_likes_article_anonymous,priority:2;size:120;not null" json:"anonymous_id"`
	IPAddress   string    `gorm:"size:64;not null" json:"ip_address"`
	UserAgent   string    `gorm:"size:255;not null;default:''" json:"user_agent"`
	CreatedAt   time.Time `json:"created_at"`
}

type Bookmark struct {
	ID          uint      `gorm:"primaryKey" json:"id"`
	ArticleID   uint      `gorm:"uniqueIndex:uq_bookmarks_article_anonymous,priority:1;index;not null" json:"article_id"`
	Article     *Article  `json:"article,omitempty"`
	AnonymousID string    `gorm:"uniqueIndex:uq_bookmarks_article_anonymous,priority:2;size:120;not null" json:"anonymous_id"`
	IPAddress   string    `gorm:"size:64;not null;default:''" json:"ip_address"`
	UserAgent   string    `gorm:"size:255;not null;default:''" json:"user_agent"`
	CreatedAt   time.Time `json:"created_at"`
}

type AuthorFollow struct {
	ID          uint      `gorm:"primaryKey" json:"id"`
	AuthorID    uint      `gorm:"uniqueIndex:uq_author_follows_author_anonymous,priority:1;index;not null" json:"author_id"`
	Author      *User     `json:"author,omitempty"`
	AnonymousID string    `gorm:"uniqueIndex:uq_author_follows_author_anonymous,priority:2;size:120;not null" json:"anonymous_id"`
	IPAddress   string    `gorm:"size:64;not null;default:''" json:"ip_address"`
	UserAgent   string    `gorm:"size:255;not null;default:''" json:"user_agent"`
	CreatedAt   time.Time `json:"created_at"`
}

type NewsletterSubscription struct {
	ID          uint      `gorm:"primaryKey" json:"id"`
	Email       string    `gorm:"uniqueIndex;size:255;not null" json:"email"`
	AnonymousID string    `gorm:"size:120;not null;default:''" json:"anonymous_id"`
	Status      string    `gorm:"size:20;not null;default:subscribed" json:"status"`
	IPAddress   string    `gorm:"size:64;not null;default:''" json:"ip_address"`
	UserAgent   string    `gorm:"size:255;not null;default:''" json:"user_agent"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`
}

type SlugHistory struct {
	ID        uint      `gorm:"primaryKey" json:"id"`
	ArticleID *uint     `gorm:"index" json:"article_id"`
	OldSlug   string    `gorm:"uniqueIndex;size:160;not null" json:"old_slug"`
	CreatedAt time.Time `json:"created_at"`
}

func (SlugHistory) TableName() string {
	return "slug_history"
}
