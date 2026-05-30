package testutil

import (
	"bytes"
	"context"
	"database/sql"
	"encoding/base64"
	"image"
	"image/color"
	"image/png"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"blogWeb/config"
	"blogWeb/internal/model"
	"blogWeb/internal/service"

	"github.com/alicebob/miniredis/v2"
	"github.com/glebarez/sqlite"
	"github.com/redis/go-redis/v9"
	"gorm.io/gorm"
)

type App struct {
	Config      *config.Config
	DB          *gorm.DB
	Redis       *redis.Client
	Renderer    *service.RendererService
	RateLimiter *service.RateLimiter
	Sessions    *service.SessionManager
	Auth        *service.AuthService
	Categories  *service.CategoryService
	Articles    *service.ArticleService
	Comments    *service.CommentService
	Likes       *service.LikeService
	Uploads     *service.UploadService
	MiniRedis   *miniredis.Miniredis
}

func NewApp(t *testing.T) *App {
	t.Helper()

	cfg := config.Default()
	tempDir := t.TempDir()
	cfg.Database.Path = filepath.Join(tempDir, "test.db")
	cfg.Upload.Dir = filepath.Join(tempDir, "uploads")
	cfg.Admin.InitPassword = "admin-password"
	cfg.Session.Secret = "test-session-secret-1234567890"
	cfg.Site.BaseURL = "http://example.test"

	db, err := gorm.Open(sqlite.Open(cfg.Database.Path), &gorm.Config{})
	if err != nil {
		t.Fatalf("open sqlite: %v", err)
	}
	sqlDB, err := db.DB()
	if err != nil {
		t.Fatalf("db handle: %v", err)
	}
	t.Cleanup(func() {
		_ = sqlDB.Close()
	})
	runMigrations(t, sqlDB)

	renderer := service.NewRendererService()
	miniRedis, err := miniredis.Run()
	if err != nil {
		t.Fatalf("start miniredis: %v", err)
	}
	t.Cleanup(miniRedis.Close)
	rdb := redis.NewClient(&redis.Options{Addr: miniRedis.Addr()})
	t.Cleanup(func() { _ = rdb.Close() })
	limiter := service.NewRateLimiter(rdb)
	sessions := service.NewSessionManager(rdb, cfg.Session)
	auth := service.NewAuthService(db, cfg.Admin, sessions, limiter, cfg.Email)
	if err := auth.EnsureInitialAdmin(context.Background()); err != nil {
		t.Fatalf("ensure admin: %v", err)
	}
	categories := service.NewCategoryService(db, renderer)
	articles := service.NewArticleService(db, renderer)
	comments := service.NewCommentService(db)
	likes := service.NewLikeService(db)
	uploads := service.NewUploadService(cfg.Upload)

	return &App{
		Config:      cfg,
		DB:          db,
		Redis:       rdb,
		Renderer:    renderer,
		RateLimiter: limiter,
		Sessions:    sessions,
		Auth:        auth,
		Categories:  categories,
		Articles:    articles,
		Comments:    comments,
		Likes:       likes,
		Uploads:     uploads,
		MiniRedis:   miniRedis,
	}
}

func runMigrations(t *testing.T, db *sql.DB) {
	t.Helper()
	_, currentFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatalf("resolve runtime caller")
	}
	projectRoot := filepath.Clean(filepath.Join(filepath.Dir(currentFile), "..", ".."))
	files := []string{
		filepath.Join(projectRoot, "migrations", "001_init.sql"),
		filepath.Join(projectRoot, "migrations", "002_mcp.sql"),
		filepath.Join(projectRoot, "migrations", "003_comments.sql"),
		filepath.Join(projectRoot, "migrations", "004_reader_interactions.sql"),
		filepath.Join(projectRoot, "migrations", "005_email_registration.sql"),
	}
	for _, file := range files {
		data, err := os.ReadFile(file)
		if err != nil {
			t.Fatalf("read migration %s: %v", file, err)
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
				t.Fatalf("exec migration %s: %v", file, err)
			}
		}
	}
}

func CreateAdminSessionCookie(t *testing.T, app *App) (string, string) {
	t.Helper()
	password, err := app.Auth.Login(context.Background(), "127.0.0.1", app.Config.RateLimit, app.Config.Admin.InitUsername, app.Config.Admin.InitPassword)
	if err != nil {
		t.Fatalf("login admin: %v", err)
	}
	return password.SessionID, password.Session.CSRFToken
}

func SeedCategory(t *testing.T, app *App, name, slug string) *model.Category {
	t.Helper()
	category, err := app.Categories.Create(context.Background(), service.CreateCategoryInput{
		Name: name,
		Slug: slug,
	})
	if err != nil {
		t.Fatalf("seed category: %v", err)
	}
	return category
}

func SeedArticle(t *testing.T, app *App, input service.CreateArticleInput) *model.Article {
	t.Helper()
	if input.AuthorID == 0 {
		var admin model.User
		if err := app.DB.Where("username = ?", app.Config.Admin.InitUsername).First(&admin).Error; err != nil {
			t.Fatalf("query admin: %v", err)
		}
		input.AuthorID = admin.ID
	}
	article, err := app.Articles.Create(context.Background(), input)
	if err != nil {
		t.Fatalf("seed article: %v", err)
	}
	return article
}

func MustPNGBase64(t *testing.T) string {
	t.Helper()
	img := image.NewRGBA(image.Rect(0, 0, 1, 1))
	img.Set(0, 0, color.RGBA{R: 255, A: 255})
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		t.Fatalf("encode png: %v", err)
	}
	return base64.StdEncoding.EncodeToString(buf.Bytes())
}
