package app

import (
	"context"
	"database/sql"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"blogWeb/config"
	"blogWeb/internal/handler"
	"blogWeb/internal/mcp"
	"blogWeb/internal/seed"
	"blogWeb/internal/service"

	"github.com/gin-gonic/gin"
	"github.com/glebarez/sqlite"
	"github.com/redis/go-redis/v9"
	"gorm.io/gorm"
)

type Application struct {
	Config      *config.Config
	Logger      *slog.Logger
	DB          *gorm.DB
	Redis       *redis.Client
	Renderer    *service.RendererService
	RateLimiter *service.RateLimiter
	Auth        *service.AuthService
	Categories  *service.CategoryService
	Articles    *service.ArticleService
	Comments    *service.CommentService
	Likes       *service.LikeService
	Uploads     *service.UploadService
	Sessions    *service.SessionManager
	Handler     *handler.HTTPHandler
	MCP         *mcp.Server
}

func Bootstrap(ctx context.Context, configPath string, requireRedis bool) (*Application, error) {
	cfg, err := config.Load(configPath)
	if err != nil {
		return nil, err
	}

	logger := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelInfo}))

	if err := os.MkdirAll(filepath.Dir(cfg.Database.Path), 0o755); err != nil {
		return nil, fmt.Errorf("create database dir: %w", err)
	}
	if err := os.MkdirAll(cfg.Upload.Dir, 0o755); err != nil {
		return nil, fmt.Errorf("create upload dir: %w", err)
	}

	db, err := gorm.Open(sqlite.Open(cfg.Database.Path), &gorm.Config{})
	if err != nil {
		return nil, fmt.Errorf("open database: %w", err)
	}
	sqlDB, err := db.DB()
	if err != nil {
		return nil, fmt.Errorf("database handle: %w", err)
	}
	if err := sqlDB.PingContext(ctx); err != nil {
		return nil, fmt.Errorf("ping database: %w", err)
	}
	if err := runMigrations(sqlDB); err != nil {
		return nil, err
	}

	redisClient := redis.NewClient(&redis.Options{
		Addr:     cfg.Redis.Addr,
		Password: cfg.Redis.Password,
		DB:       cfg.Redis.DB,
		PoolSize: cfg.Redis.PoolSize,
	})
	if requireRedis {
		if err := redisClient.Ping(ctx).Err(); err != nil {
			return nil, fmt.Errorf("ping redis: %w", err)
		}
	}

	renderer := service.NewRendererService()
	rateLimiter := service.NewRateLimiter(redisClient)
	sessions := service.NewSessionManager(redisClient, cfg.Session)
	auth := service.NewAuthService(db, cfg.Admin, sessions, rateLimiter, cfg.Email)
	categories := service.NewCategoryService(db, renderer)
	articles := service.NewArticleService(db, renderer)
	comments := service.NewCommentService(db)
	likes := service.NewLikeService(db)
	uploads := service.NewUploadService(cfg.Upload)

	if err := auth.EnsureInitialAdmin(ctx); err != nil {
		return nil, fmt.Errorf("ensure admin: %w", err)
	}
	if err := seed.EnsureDemoContent(ctx, db, auth, articles, categories); err != nil {
		return nil, fmt.Errorf("ensure demo content: %w", err)
	}

	application := &Application{
		Config:      cfg,
		Logger:      logger,
		DB:          db,
		Redis:       redisClient,
		Renderer:    renderer,
		RateLimiter: rateLimiter,
		Auth:        auth,
		Categories:  categories,
		Articles:    articles,
		Comments:    comments,
		Likes:       likes,
		Uploads:     uploads,
		Sessions:    sessions,
	}
	application.Handler = handler.NewHTTPHandler(
		cfg,
		auth,
		categories,
		articles,
		likes,
		uploads,
		sessions,
		rateLimiter,
		comments,
	)
	application.MCP = mcp.NewServer(
		cfg,
		logger,
		db,
		rateLimiter,
		renderer,
		categories,
		articles,
		uploads,
	)
	return application, nil
}

func (a *Application) WebRouter() *gin.Engine {
	return a.Handler.Router()
}

func (a *Application) MCPHTTPHandler() http.Handler {
	return a.MCP.HTTPHandler()
}

func runMigrations(db *sql.DB) error {
	files := []string{
		filepath.Join("migrations", "001_init.sql"),
		filepath.Join("migrations", "002_mcp.sql"),
		filepath.Join("migrations", "003_comments.sql"),
		filepath.Join("migrations", "004_reader_interactions.sql"),
		filepath.Join("migrations", "005_email_registration.sql"),
	}
	for _, file := range files {
		data, err := os.ReadFile(file)
		if err != nil {
			return fmt.Errorf("read migration %s: %w", file, err)
		}
		for _, statement := range strings.Split(string(data), ";") {
			statement = strings.TrimSpace(statement)
			if statement == "" {
				continue
			}
			if _, err := db.Exec(statement); err != nil {
				if strings.Contains(strings.ToLower(err.Error()), "duplicate column name") {
					continue
				}
				return fmt.Errorf("exec migration %s: %w", file, err)
			}
		}
	}
	return nil
}
