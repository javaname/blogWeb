package service

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"blogWeb/config"

	"github.com/redis/go-redis/v9"
)

const AdminSessionCookieName = "admin_session"

type SessionUser struct {
	UserID    uint      `json:"user_id"`
	Username  string    `json:"username"`
	Role      string    `json:"role"`
	CSRFToken string    `json:"csrf_token"`
	CreatedAt time.Time `json:"created_at"`
	LastSeen  time.Time `json:"last_seen"`
}

type SessionManager struct {
	redis  *redis.Client
	config config.SessionConfig
}

func NewSessionManager(redisClient *redis.Client, cfg config.SessionConfig) *SessionManager {
	return &SessionManager{
		redis:  redisClient,
		config: cfg,
	}
}

func (m *SessionManager) Create(ctx context.Context, userID uint, username, role string) (string, *SessionUser, error) {
	if m.redis == nil {
		return "", nil, NewAppError(500, "redis_unavailable", "会话存储不可用")
	}

	sessionID, err := NewToken(24)
	if err != nil {
		return "", nil, err
	}
	csrfToken, err := NewToken(24)
	if err != nil {
		return "", nil, err
	}
	now := time.Now().UTC()
	session := &SessionUser{
		UserID:    userID,
		Username:  username,
		Role:      role,
		CSRFToken: csrfToken,
		CreatedAt: now,
		LastSeen:  now,
	}
	if err := m.save(ctx, sessionID, session); err != nil {
		return "", nil, err
	}
	return sessionID, session, nil
}

func (m *SessionManager) Get(ctx context.Context, sessionID string) (*SessionUser, error) {
	if m.redis == nil || sessionID == "" {
		return nil, nil
	}
	raw, err := m.redis.Get(ctx, m.sessionKey(sessionID)).Result()
	if err == redis.Nil {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	var session SessionUser
	if err := json.Unmarshal([]byte(raw), &session); err != nil {
		return nil, err
	}
	now := time.Now().UTC()
	if now.Sub(session.CreatedAt) > time.Duration(m.config.MaxAge)*time.Second {
		_ = m.Destroy(ctx, sessionID)
		return nil, nil
	}
	if now.Sub(session.LastSeen) > time.Duration(m.config.IdleTimeout)*time.Second {
		_ = m.Destroy(ctx, sessionID)
		return nil, nil
	}
	session.LastSeen = now
	if err := m.save(ctx, sessionID, &session); err != nil {
		return nil, err
	}
	return &session, nil
}

func (m *SessionManager) Destroy(ctx context.Context, sessionID string) error {
	if m.redis == nil || sessionID == "" {
		return nil
	}
	return m.redis.Del(ctx, m.sessionKey(sessionID), m.csrfKey(sessionID)).Err()
}

func (m *SessionManager) CSRFToken(ctx context.Context, sessionID string) (string, error) {
	session, err := m.Get(ctx, sessionID)
	if err != nil || session == nil {
		return "", err
	}
	return session.CSRFToken, nil
}

func (m *SessionManager) ValidateCSRF(ctx context.Context, sessionID, token string) (bool, error) {
	if sessionID == "" || token == "" {
		return false, nil
	}
	session, err := m.Get(ctx, sessionID)
	if err != nil || session == nil {
		return false, err
	}
	return session.CSRFToken == token, nil
}

func (m *SessionManager) save(ctx context.Context, sessionID string, session *SessionUser) error {
	data, err := json.Marshal(session)
	if err != nil {
		return err
	}
	ttl := time.Duration(m.config.MaxAge) * time.Second
	pipe := m.redis.TxPipeline()
	pipe.Set(ctx, m.sessionKey(sessionID), data, ttl)
	pipe.Set(ctx, m.csrfKey(sessionID), session.CSRFToken, ttl)
	_, err = pipe.Exec(ctx)
	return err
}

func (m *SessionManager) sessionKey(sessionID string) string {
	return fmt.Sprintf("session:%s", sessionID)
}

func (m *SessionManager) csrfKey(sessionID string) string {
	return fmt.Sprintf("csrf:%s", sessionID)
}
