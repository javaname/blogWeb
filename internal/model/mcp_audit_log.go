package model

import "time"

type MCPAuditLog struct {
	ID            uint      `gorm:"primaryKey" json:"id"`
	ClientID      *uint     `gorm:"index" json:"client_id"`
	Transport     string    `gorm:"size:20;not null" json:"transport"`
	ActionType    string    `gorm:"size:50;index;not null" json:"action_type"`
	Target        string    `gorm:"size:255;not null" json:"target"`
	Scope         string    `gorm:"size:120;not null;default:''" json:"scope"`
	Status        string    `gorm:"size:50;not null" json:"status"`
	RequestID     string    `gorm:"size:120;not null;default:''" json:"request_id"`
	ActorIP       string    `gorm:"size:120;not null;default:''" json:"actor_ip"`
	ErrorCode     string    `gorm:"size:120;not null;default:''" json:"error_code"`
	PayloadDigest string    `gorm:"size:255;not null;default:''" json:"payload_digest"`
	CreatedAt     time.Time `gorm:"index" json:"created_at"`
}
