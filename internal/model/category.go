package model

import "time"

type Category struct {
	ID        uint      `gorm:"primaryKey" json:"id"`
	Name      string    `gorm:"uniqueIndex;size:120;not null" json:"name"`
	Slug      string    `gorm:"uniqueIndex;size:160;not null" json:"slug"`
	SortOrder int       `gorm:"not null;default:0" json:"sort_order"`
	CreatedAt time.Time `json:"created_at"`
}
