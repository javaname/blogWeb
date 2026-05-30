package model

import "time"

type MCPClient struct {
	ID         uint       `gorm:"primaryKey" json:"id"`
	Name       string     `gorm:"uniqueIndex;size:120;not null" json:"name"`
	TokenHash  string     `gorm:"uniqueIndex;size:255;not null" json:"-"`
	Scopes     string     `gorm:"type:text;not null" json:"scopes"`
	Transport  string     `gorm:"size:20;not null;default:http" json:"transport"`
	IsEnabled  bool       `gorm:"index;not null;default:true" json:"is_enabled"`
	CreatedBy  *uint      `json:"created_by"`
	LastUsedAt *time.Time `json:"last_used_at"`
	CreatedAt  time.Time  `json:"created_at"`
	UpdatedAt  time.Time  `json:"updated_at"`
}
