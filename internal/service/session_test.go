package service

import (
	"context"
	"encoding/json"
	"testing"
	"time"

	"blogWeb/config"

	"github.com/alicebob/miniredis/v2"
	"github.com/redis/go-redis/v9"
)

func TestSessionManagerCreateGetAndValidateCSRF(t *testing.T) {
	t.Parallel()

	mini, err := miniredis.Run()
	if err != nil {
		t.Fatalf("start miniredis: %v", err)
	}
	defer mini.Close()

	rdb := redis.NewClient(&redis.Options{Addr: mini.Addr()})
	manager := NewSessionManager(rdb, config.SessionConfig{
		Secret:      "test",
		MaxAge:      60,
		IdleTimeout: 60,
	})

	sessionID, session, err := manager.Create(context.Background(), 1, "admin", "admin")
	if err != nil {
		t.Fatalf("create session: %v", err)
	}
	if sessionID == "" || session.CSRFToken == "" {
		t.Fatalf("expected session and csrf token")
	}

	got, err := manager.Get(context.Background(), sessionID)
	if err != nil {
		t.Fatalf("get session: %v", err)
	}
	if got == nil || got.Username != "admin" {
		t.Fatalf("unexpected session: %+v", got)
	}

	valid, err := manager.ValidateCSRF(context.Background(), sessionID, session.CSRFToken)
	if err != nil {
		t.Fatalf("validate csrf: %v", err)
	}
	if !valid {
		t.Fatalf("expected csrf valid")
	}
}

func TestSessionManagerExpiresIdleSession(t *testing.T) {
	t.Parallel()

	mini, err := miniredis.Run()
	if err != nil {
		t.Fatalf("start miniredis: %v", err)
	}
	defer mini.Close()

	rdb := redis.NewClient(&redis.Options{Addr: mini.Addr()})
	manager := NewSessionManager(rdb, config.SessionConfig{
		Secret:      "test",
		MaxAge:      3600,
		IdleTimeout: 1,
	})

	sessionID, _, err := manager.Create(context.Background(), 1, "admin", "admin")
	if err != nil {
		t.Fatalf("create session: %v", err)
	}

	raw, err := rdb.Get(context.Background(), "session:"+sessionID).Result()
	if err != nil {
		t.Fatalf("read session raw: %v", err)
	}
	var session SessionUser
	if err := json.Unmarshal([]byte(raw), &session); err != nil {
		t.Fatalf("unmarshal session: %v", err)
	}
	session.LastSeen = time.Now().UTC().Add(-2 * time.Second)
	data, err := json.Marshal(session)
	if err != nil {
		t.Fatalf("marshal session: %v", err)
	}
	if err := rdb.Set(context.Background(), "session:"+sessionID, data, time.Hour).Err(); err != nil {
		t.Fatalf("write session raw: %v", err)
	}

	got, err := manager.Get(context.Background(), sessionID)
	if err != nil {
		t.Fatalf("get expired session: %v", err)
	}
	if got != nil {
		t.Fatalf("expected idle-expired session to be removed")
	}
}
