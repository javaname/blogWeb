package service

import (
	"context"
	"fmt"
	"time"

	"github.com/redis/go-redis/v9"
)

type RateLimiter struct {
	redis *redis.Client
}

func NewRateLimiter(redisClient *redis.Client) *RateLimiter {
	return &RateLimiter{redis: redisClient}
}

func (r *RateLimiter) Allow(ctx context.Context, key string, maxAttempts int, window time.Duration) (bool, int64, error) {
	if r.redis == nil {
		return true, 0, nil
	}

	count, err := r.redis.Incr(ctx, key).Result()
	if err != nil {
		return false, 0, err
	}
	if count == 1 {
		if err := r.redis.Expire(ctx, key, window).Err(); err != nil {
			return false, count, err
		}
	}
	return count <= int64(maxAttempts), count, nil
}

func (r *RateLimiter) Get(ctx context.Context, key string) (int64, error) {
	if r.redis == nil {
		return 0, nil
	}
	value, err := r.redis.Get(ctx, key).Int64()
	if err == redis.Nil {
		return 0, nil
	}
	return value, err
}

func (r *RateLimiter) Reset(ctx context.Context, keys ...string) error {
	if r.redis == nil || len(keys) == 0 {
		return nil
	}
	return r.redis.Del(ctx, keys...).Err()
}

func LoginRateKey(ip string) string {
	return fmt.Sprintf("login_rate:%s", ip)
}

func LoginFailKey(username string) string {
	return fmt.Sprintf("login_fail:%s", username)
}

func RegistrationRateKey(ip string) string {
	return fmt.Sprintf("registration_rate:%s", ip)
}

func RegistrationEmailRateKey(email string) string {
	return fmt.Sprintf("registration_email_rate:%s", email)
}

func LikeRateKey(ip string) string {
	return fmt.Sprintf("like_rate:%s", ip)
}

func LikeArticleRateKey(ip string, articleID uint) string {
	return fmt.Sprintf("like_article_rate:%s:%d", ip, articleID)
}

func CommentRateKey(ip string) string {
	return fmt.Sprintf("comment_rate:%s", ip)
}

func CommentArticleRateKey(ip string, articleID uint) string {
	return fmt.Sprintf("comment_article_rate:%s:%d", ip, articleID)
}

func MCPReadRateKey(clientID uint) string {
	return fmt.Sprintf("mcp_read_rate:%d", clientID)
}

func MCPWriteRateKey(clientID uint) string {
	return fmt.Sprintf("mcp_write_rate:%d", clientID)
}

func MCPUploadRateKey(clientID uint) string {
	return fmt.Sprintf("mcp_upload_rate:%d", clientID)
}
