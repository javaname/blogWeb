package config

import (
	"errors"
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

type Config struct {
	Server    ServerConfig    `yaml:"server"`
	Database  DatabaseConfig  `yaml:"database"`
	Redis     RedisConfig     `yaml:"redis"`
	Session   SessionConfig   `yaml:"session"`
	RateLimit RateLimitConfig `yaml:"rate_limit"`
	Upload    UploadConfig    `yaml:"upload"`
	Admin     AdminConfig     `yaml:"admin"`
	Email     EmailConfig     `yaml:"email"`
	MCP       MCPConfig       `yaml:"mcp"`
	Site      SiteConfig      `yaml:"site"`
}

type ServerConfig struct {
	Port int `yaml:"port"`
}

type DatabaseConfig struct {
	Path string `yaml:"path"`
}

type RedisConfig struct {
	Addr     string `yaml:"addr"`
	Password string `yaml:"password"`
	DB       int    `yaml:"db"`
	PoolSize int    `yaml:"pool_size"`
}

type SessionConfig struct {
	Secret      string `yaml:"secret"`
	MaxAge      int    `yaml:"max_age"`
	IdleTimeout int    `yaml:"idle_timeout"`
}

type RateLimitConfig struct {
	LoginIPWindowSec             int `yaml:"login_ip_window_sec"`
	LoginIPMaxAttempts           int `yaml:"login_ip_max_attempts"`
	LoginUserFailThreshold       int `yaml:"login_user_fail_threshold"`
	LoginUserCooldownSec         int `yaml:"login_user_cooldown_sec"`
	RegistrationIPWindowSec      int `yaml:"registration_ip_window_sec"`
	RegistrationIPMaxRequests    int `yaml:"registration_ip_max_requests"`
	RegistrationEmailWindowSec   int `yaml:"registration_email_window_sec"`
	RegistrationEmailMaxRequests int `yaml:"registration_email_max_requests"`
	LikeIPWindowSec              int `yaml:"like_ip_window_sec"`
	LikeIPMaxRequests            int `yaml:"like_ip_max_requests"`
	LikeArticleWindowSec         int `yaml:"like_article_window_sec"`
	LikeArticleMaxActions        int `yaml:"like_article_max_actions"`
	CommentIPWindowSec           int `yaml:"comment_ip_window_sec"`
	CommentIPMaxRequests         int `yaml:"comment_ip_max_requests"`
	CommentArticleWindowSec      int `yaml:"comment_article_window_sec"`
	CommentArticleMaxActions     int `yaml:"comment_article_max_actions"`
}

type UploadConfig struct {
	Dir          string   `yaml:"dir"`
	MaxSize      int64    `yaml:"max_size"`
	AllowedTypes []string `yaml:"allowed_types"`
	AllowSVG     bool     `yaml:"allow_svg"`
	Reencode     bool     `yaml:"reencode"`
}

type AdminConfig struct {
	InitUsername string `yaml:"init_username"`
	InitPassword string `yaml:"init_password"`
}

type EmailConfig struct {
	SMTPHost           string `yaml:"smtp_host"`
	SMTPPort           int    `yaml:"smtp_port"`
	Username           string `yaml:"username"`
	Password           string `yaml:"password"`
	From               string `yaml:"from"`
	VerificationTTLSec int    `yaml:"verification_ttl_sec"`
}

type SiteConfig struct {
	Title       string `yaml:"title"`
	Description string `yaml:"description"`
	BaseURL     string `yaml:"base_url"`
}

type MCPConfig struct {
	Enabled            bool               `yaml:"enabled"`
	StdioEnabled       bool               `yaml:"stdio_enabled"`
	StdioWriteEnabled  bool               `yaml:"stdio_write_enabled"`
	HTTPEnabled        bool               `yaml:"http_enabled"`
	HTTPAddr           string             `yaml:"http_addr"`
	HTTPPath           string             `yaml:"http_path"`
	AuthMode           string             `yaml:"auth_mode"`
	RequireOriginCheck bool               `yaml:"require_origin_check"`
	AllowedOrigins     []string           `yaml:"allowed_origins"`
	StatelessHTTP      bool               `yaml:"stateless_http"`
	ProtocolVersions   []string           `yaml:"protocol_versions"`
	RateLimit          MCPRateLimitConfig `yaml:"rate_limit"`
}

type MCPRateLimitConfig struct {
	ReadPerMinute   int `yaml:"read_per_minute"`
	WritePerMinute  int `yaml:"write_per_minute"`
	PublishPer10Min int `yaml:"publish_per_10min"`
	UploadPer10Min  int `yaml:"upload_per_10min"`
}

func Default() *Config {
	return &Config{
		Server: ServerConfig{
			Port: 3000,
		},
		Database: DatabaseConfig{
			Path: "data/blog.db",
		},
		Redis: RedisConfig{
			Addr:     "127.0.0.1:6379",
			DB:       0,
			PoolSize: 10,
		},
		Session: SessionConfig{
			Secret:      "change-this-session-secret-to-32-bytes",
			MaxAge:      86400,
			IdleTimeout: 7200,
		},
		RateLimit: RateLimitConfig{
			LoginIPWindowSec:             600,
			LoginIPMaxAttempts:           20,
			LoginUserFailThreshold:       5,
			LoginUserCooldownSec:         900,
			RegistrationIPWindowSec:      600,
			RegistrationIPMaxRequests:    5,
			RegistrationEmailWindowSec:   600,
			RegistrationEmailMaxRequests: 3,
			LikeIPWindowSec:              60,
			LikeIPMaxRequests:            60,
			LikeArticleWindowSec:         600,
			LikeArticleMaxActions:        20,
			CommentIPWindowSec:           60,
			CommentIPMaxRequests:         10,
			CommentArticleWindowSec:      600,
			CommentArticleMaxActions:     5,
		},
		Upload: UploadConfig{
			Dir:     "public/uploads",
			MaxSize: 5 * 1024 * 1024,
			AllowedTypes: []string{
				"image/jpeg",
				"image/png",
				"image/gif",
				"image/webp",
			},
			AllowSVG: false,
			Reencode: true,
		},
		Admin: AdminConfig{
			InitUsername: "admin",
			InitPassword: "change-me-123456",
		},
		Email: EmailConfig{
			SMTPHost:           "smtp.163.com",
			SMTPPort:           465,
			VerificationTTLSec: 600,
		},
		Site: SiteConfig{
			Title:       "个人博客",
			Description: "一个支持后台管理与 MCP 接入的个人博客系统",
			BaseURL:     "http://localhost:3000",
		},
		MCP: MCPConfig{
			Enabled:            true,
			StdioEnabled:       true,
			StdioWriteEnabled:  false,
			HTTPEnabled:        false,
			HTTPAddr:           "127.0.0.1:3001",
			HTTPPath:           "/mcp",
			AuthMode:           "pre_shared_token",
			RequireOriginCheck: true,
			AllowedOrigins: []string{
				"https://chatgpt.com",
				"https://chat.openai.com",
			},
			StatelessHTTP:    true,
			ProtocolVersions: []string{"2025-11-25"},
			RateLimit: MCPRateLimitConfig{
				ReadPerMinute:   120,
				WritePerMinute:  30,
				PublishPer10Min: 10,
				UploadPer10Min:  10,
			},
		},
	}
}

func Load(path string) (*Config, error) {
	cfg := Default()
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read config: %w", err)
	}
	if err := yaml.Unmarshal(data, cfg); err != nil {
		return nil, fmt.Errorf("parse config: %w", err)
	}
	if err := cfg.Validate(); err != nil {
		return nil, err
	}
	return cfg, nil
}

func (c *Config) Validate() error {
	if c.Server.Port <= 0 {
		return errors.New("server.port must be greater than 0")
	}
	if c.Database.Path == "" {
		return errors.New("database.path is required")
	}
	if c.Redis.Addr == "" {
		return errors.New("redis.addr is required")
	}
	if c.Session.Secret == "" {
		return errors.New("session.secret is required")
	}
	if c.Session.MaxAge <= 0 {
		return errors.New("session.max_age must be greater than 0")
	}
	if c.Session.IdleTimeout <= 0 {
		return errors.New("session.idle_timeout must be greater than 0")
	}
	if c.Upload.Dir == "" {
		return errors.New("upload.dir is required")
	}
	if c.Upload.MaxSize <= 0 {
		return errors.New("upload.max_size must be greater than 0")
	}
	if c.Admin.InitUsername == "" {
		return errors.New("admin.init_username is required")
	}
	if c.Admin.InitPassword == "" {
		return errors.New("admin.init_password is required")
	}
	if c.Email.SMTPPort == 0 {
		c.Email.SMTPPort = 465
	}
	if c.Email.VerificationTTLSec == 0 {
		c.Email.VerificationTTLSec = 600
	}
	if c.RateLimit.RegistrationIPWindowSec == 0 {
		c.RateLimit.RegistrationIPWindowSec = 600
	}
	if c.RateLimit.RegistrationIPMaxRequests == 0 {
		c.RateLimit.RegistrationIPMaxRequests = 5
	}
	if c.RateLimit.RegistrationEmailWindowSec == 0 {
		c.RateLimit.RegistrationEmailWindowSec = 600
	}
	if c.RateLimit.RegistrationEmailMaxRequests == 0 {
		c.RateLimit.RegistrationEmailMaxRequests = 3
	}
	if c.MCP.HTTPAddr == "" {
		c.MCP.HTTPAddr = "127.0.0.1:3001"
	}
	if c.MCP.HTTPPath == "" {
		c.MCP.HTTPPath = "/mcp"
	}
	if len(c.MCP.ProtocolVersions) == 0 {
		c.MCP.ProtocolVersions = []string{"2025-11-25"}
	}
	if c.MCP.AuthMode == "" {
		c.MCP.AuthMode = "pre_shared_token"
	}
	return nil
}
