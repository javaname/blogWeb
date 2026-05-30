package model

import "time"

type User struct {
	ID              uint       `gorm:"primaryKey" json:"id"`
	Username        string     `gorm:"uniqueIndex;size:120;not null" json:"username"`
	Email           string     `gorm:"uniqueIndex;size:255" json:"email,omitempty"`
	EmailVerifiedAt *time.Time `json:"email_verified_at,omitempty"`
	Password        string     `gorm:"size:255;not null" json:"-"`
	Role            string     `gorm:"size:20;not null;default:user" json:"role"`
	CreatedAt       time.Time  `json:"created_at"`
}

type EmailVerificationCode struct {
	ID        uint       `gorm:"primaryKey" json:"id"`
	Email     string     `gorm:"index;size:255;not null" json:"email"`
	CodeHash  string     `gorm:"size:255;not null" json:"-"`
	ExpiresAt time.Time  `gorm:"index;not null" json:"expires_at"`
	UsedAt    *time.Time `gorm:"index" json:"used_at,omitempty"`
	CreatedAt time.Time  `json:"created_at"`
}
