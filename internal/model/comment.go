package model

import "time"

type Comment struct {
	ID              uint      `gorm:"primaryKey" json:"id"`
	ArticleID       uint      `gorm:"index;not null" json:"article_id"`
	Article         *Article  `json:"article,omitempty"`
	ParentID        *uint     `gorm:"index" json:"parent_id"`
	AuthorName      string    `gorm:"size:80;not null" json:"author_name"`
	Content         string    `gorm:"type:text;not null" json:"content"`
	Status          string    `gorm:"size:20;not null;default:approved" json:"status"`
	RejectionReason string    `gorm:"type:text;not null;default:''" json:"rejection_reason"`
	AnonymousID     string    `gorm:"size:120;not null;default:''" json:"anonymous_id"`
	IPAddress       string    `gorm:"size:64;not null;default:''" json:"ip_address"`
	UserAgent       string    `gorm:"size:255;not null;default:''" json:"user_agent"`
	CreatedAt       time.Time `json:"created_at"`
	UpdatedAt       time.Time `json:"updated_at"`
}
