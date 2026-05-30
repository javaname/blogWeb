package service

import (
	"context"
	"crypto/rand"
	"errors"
	"fmt"
	"math/big"
	"net/mail"
	"strings"
	"time"

	"blogWeb/config"
	"blogWeb/internal/model"

	"golang.org/x/crypto/bcrypt"
	"gorm.io/gorm"
)

type AuthService struct {
	db                      *gorm.DB
	adminConfig             config.AdminConfig
	emailConfig             config.EmailConfig
	sessions                *SessionManager
	limiter                 *RateLimiter
	registrationEmailSender RegistrationEmailSender
}

type LoginResult struct {
	SessionID string
	Session   *SessionUser
	User      *model.User
}

type RegistrationCodeResult struct {
	Sent      bool `json:"sent"`
	ExpiresIn int  `json:"expires_in"`
}

type RegisterWithEmailInput struct {
	Email           string
	Code            string
	Password        string
	ConfirmPassword string
}

func NewAuthService(db *gorm.DB, adminConfig config.AdminConfig, sessions *SessionManager, limiter *RateLimiter, emailConfig ...config.EmailConfig) *AuthService {
	cfg := config.EmailConfig{VerificationTTLSec: 600}
	if len(emailConfig) > 0 {
		cfg = emailConfig[0]
	}
	if cfg.VerificationTTLSec <= 0 {
		cfg.VerificationTTLSec = 600
	}
	service := &AuthService{
		db:          db,
		adminConfig: adminConfig,
		emailConfig: cfg,
		sessions:    sessions,
		limiter:     limiter,
	}
	service.registrationEmailSender = NewSMTPRegistrationEmailSender(cfg)
	return service
}

func (s *AuthService) SetRegistrationEmailSender(sender RegistrationEmailSender) {
	s.registrationEmailSender = sender
}

func (s *AuthService) RequestRegistrationCode(ctx context.Context, ip string, rateConfig config.RateLimitConfig, email string) (*RegistrationCodeResult, error) {
	email, err := normalizeEmail(email)
	if err != nil {
		return nil, err
	}
	if err := s.ensureEmailNotRegistered(ctx, email); err != nil {
		return nil, err
	}

	if s.limiter != nil {
		ipWindow := time.Duration(defaultInt(rateConfig.RegistrationIPWindowSec, 600)) * time.Second
		emailWindow := time.Duration(defaultInt(rateConfig.RegistrationEmailWindowSec, 600)) * time.Second
		allowed, _, err := s.limiter.Allow(ctx, RegistrationRateKey(ip), defaultInt(rateConfig.RegistrationIPMaxRequests, 5), ipWindow)
		if err != nil {
			return nil, err
		}
		if !allowed {
			return nil, NewAppError(429, "rate_limited", "注册请求过于频繁，请稍后再试")
		}
		allowed, _, err = s.limiter.Allow(ctx, RegistrationEmailRateKey(email), defaultInt(rateConfig.RegistrationEmailMaxRequests, 3), emailWindow)
		if err != nil {
			return nil, err
		}
		if !allowed {
			return nil, NewAppError(429, "rate_limited", "注册请求过于频繁，请稍后再试")
		}
	}

	if s.registrationEmailSender == nil {
		return nil, NewAppError(500, "email_unavailable", "邮箱服务尚未配置")
	}
	code, err := newVerificationCode()
	if err != nil {
		return nil, err
	}
	ttlSec := defaultInt(s.emailConfig.VerificationTTLSec, 600)
	ttl := time.Duration(ttlSec) * time.Second
	if err := s.registrationEmailSender.SendRegistrationCode(ctx, email, code, ttl); err != nil {
		return nil, err
	}
	if err := s.db.WithContext(ctx).Create(&model.EmailVerificationCode{
		Email:     email,
		CodeHash:  verificationCodeHash(email, code),
		ExpiresAt: time.Now().UTC().Add(ttl),
	}).Error; err != nil {
		return nil, err
	}
	return &RegistrationCodeResult{Sent: true, ExpiresIn: ttlSec}, nil
}

func (s *AuthService) RegisterWithEmail(ctx context.Context, input RegisterWithEmailInput) (*model.User, error) {
	email, err := normalizeEmail(input.Email)
	if err != nil {
		return nil, err
	}
	code := strings.TrimSpace(input.Code)
	password := strings.TrimSpace(input.Password)
	confirmPassword := strings.TrimSpace(input.ConfirmPassword)
	if code == "" || password == "" || confirmPassword == "" {
		return nil, NewAppError(400, "invalid_params", "验证码和密码不能为空")
	}
	if password != confirmPassword {
		return nil, NewAppError(400, "invalid_params", "两次输入的密码不一致")
	}
	if len([]rune(password)) < 8 {
		return nil, NewAppError(400, "invalid_params", "密码不能少于 8 个字符")
	}

	var created model.User
	err = s.db.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		if err := s.ensureEmailNotRegisteredTx(ctx, tx, email); err != nil {
			return err
		}
		var verification model.EmailVerificationCode
		err := tx.WithContext(ctx).
			Where("email = ? AND used_at IS NULL", email).
			Order("created_at DESC, id DESC").
			First(&verification).Error
		if errors.Is(err, gorm.ErrRecordNotFound) {
			return NewAppError(400, "invalid_verification_code", "验证码错误或已过期")
		}
		if err != nil {
			return err
		}
		now := time.Now().UTC()
		if now.After(verification.ExpiresAt) || verification.CodeHash != verificationCodeHash(email, code) {
			return NewAppError(400, "invalid_verification_code", "验证码错误或已过期")
		}

		passwordHash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
		if err != nil {
			return err
		}
		username, err := s.nextRegistrationUsername(ctx, tx, email)
		if err != nil {
			return err
		}
		created = model.User{
			Username:        username,
			Email:           email,
			EmailVerifiedAt: &now,
			Password:        string(passwordHash),
			Role:            "user",
		}
		if err := tx.WithContext(ctx).Create(&created).Error; err != nil {
			return err
		}
		return tx.WithContext(ctx).Model(&verification).Update("used_at", now).Error
	})
	if err != nil {
		return nil, err
	}
	return &created, nil
}

func (s *AuthService) EnsureInitialAdmin(ctx context.Context) error {
	var count int64
	if err := s.db.WithContext(ctx).Model(&model.User{}).Where("role = ?", "admin").Count(&count).Error; err != nil {
		return err
	}
	if count > 0 {
		return nil
	}

	password, err := bcrypt.GenerateFromPassword([]byte(s.adminConfig.InitPassword), bcrypt.DefaultCost)
	if err != nil {
		return err
	}

	return s.db.WithContext(ctx).Create(&model.User{
		Username: s.adminConfig.InitUsername,
		Password: string(password),
		Role:     "admin",
	}).Error
}

func (s *AuthService) Login(ctx context.Context, ip string, rateConfig config.RateLimitConfig, username, password string) (*LoginResult, error) {
	username = strings.TrimSpace(username)
	password = strings.TrimSpace(password)
	if username == "" || password == "" {
		return nil, NewAppError(400, "invalid_params", "用户名和密码不能为空")
	}

	allowed, _, err := s.limiter.Allow(ctx, LoginRateKey(ip), rateConfig.LoginIPMaxAttempts, time.Duration(rateConfig.LoginIPWindowSec)*time.Second)
	if err != nil {
		return nil, err
	}
	if !allowed {
		return nil, NewAppError(429, "rate_limited", "登录尝试过于频繁，请稍后再试")
	}

	failures, err := s.limiter.Get(ctx, LoginFailKey(username))
	if err != nil {
		return nil, err
	}
	if failures >= int64(rateConfig.LoginUserFailThreshold) {
		return nil, NewAppError(429, "rate_limited", "登录尝试过于频繁，请稍后再试")
	}

	var user model.User
	query := s.db.WithContext(ctx)
	if strings.Contains(username, "@") {
		query = query.Where("LOWER(email) = ?", strings.ToLower(username))
	} else {
		query = query.Where("username = ?", username)
	}
	if err := query.First(&user).Error; err != nil {
		_, _, limitErr := s.limiter.Allow(ctx, LoginFailKey(username), rateConfig.LoginUserFailThreshold, time.Duration(rateConfig.LoginUserCooldownSec)*time.Second)
		if limitErr != nil {
			return nil, limitErr
		}
		return nil, NewAppError(401, "auth_failed", "用户名或密码错误")
	}
	if bcrypt.CompareHashAndPassword([]byte(user.Password), []byte(password)) != nil {
		_, _, limitErr := s.limiter.Allow(ctx, LoginFailKey(username), rateConfig.LoginUserFailThreshold, time.Duration(rateConfig.LoginUserCooldownSec)*time.Second)
		if limitErr != nil {
			return nil, limitErr
		}
		return nil, NewAppError(401, "auth_failed", "用户名或密码错误")
	}

	if err := s.limiter.Reset(ctx, LoginFailKey(username)); err != nil {
		return nil, err
	}
	sessionID, session, err := s.sessions.Create(ctx, user.ID, user.Username, user.Role)
	if err != nil {
		return nil, err
	}
	return &LoginResult{
		SessionID: sessionID,
		Session:   session,
		User:      &user,
	}, nil
}

func (s *AuthService) Logout(ctx context.Context, sessionID string) error {
	return s.sessions.Destroy(ctx, sessionID)
}

func (s *AuthService) CurrentUser(ctx context.Context, sessionID string) (*SessionUser, error) {
	return s.sessions.Get(ctx, sessionID)
}

func (s *AuthService) ensureEmailNotRegistered(ctx context.Context, email string) error {
	return s.ensureEmailNotRegisteredTx(ctx, s.db, email)
}

func (s *AuthService) ensureEmailNotRegisteredTx(ctx context.Context, tx *gorm.DB, email string) error {
	var count int64
	if err := tx.WithContext(ctx).Model(&model.User{}).Where("LOWER(email) = ?", email).Count(&count).Error; err != nil {
		return err
	}
	if count > 0 {
		return NewAppError(409, "conflict", "邮箱已注册")
	}
	return nil
}

func (s *AuthService) nextRegistrationUsername(ctx context.Context, tx *gorm.DB, email string) (string, error) {
	base := usernameBaseFromEmail(email)
	for i := 0; i < 1000; i++ {
		candidate := base
		if i > 0 {
			candidate = fmt.Sprintf("%s-%d", truncateUsernameBase(base, 116), i+1)
		}
		var count int64
		if err := tx.WithContext(ctx).Model(&model.User{}).Where("username = ?", candidate).Count(&count).Error; err != nil {
			return "", err
		}
		if count == 0 {
			return candidate, nil
		}
	}
	return "", NewAppError(409, "conflict", "无法生成可用用户名")
}

func normalizeEmail(email string) (string, error) {
	email = strings.ToLower(strings.TrimSpace(email))
	if email == "" {
		return "", NewAppError(400, "invalid_params", "邮箱不能为空")
	}
	if len([]rune(email)) > 255 {
		return "", NewAppError(400, "invalid_params", "邮箱不能超过 255 个字符")
	}
	parsed, err := mail.ParseAddress(email)
	if err != nil || strings.ToLower(parsed.Address) != email {
		return "", NewAppError(400, "invalid_params", "邮箱格式不正确")
	}
	return email, nil
}

func newVerificationCode() (string, error) {
	value, err := rand.Int(rand.Reader, big.NewInt(1000000))
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("%06d", value.Int64()), nil
}

func verificationCodeHash(email, code string) string {
	return HashDigest(email, strings.TrimSpace(code))
}

func usernameBaseFromEmail(email string) string {
	local := email
	if index := strings.Index(email, "@"); index > 0 {
		local = email[:index]
	}
	var builder strings.Builder
	for _, r := range strings.ToLower(local) {
		switch {
		case r >= 'a' && r <= 'z':
			builder.WriteRune(r)
		case r >= '0' && r <= '9':
			builder.WriteRune(r)
		case r == '-' || r == '_' || r == '.':
			builder.WriteRune(r)
		}
	}
	base := strings.Trim(builder.String(), "-_.")
	if base == "" {
		base = "user"
	}
	return truncateUsernameBase(base, 120)
}

func truncateUsernameBase(value string, max int) string {
	runes := []rune(value)
	if len(runes) <= max {
		return value
	}
	return string(runes[:max])
}

func defaultInt(value, fallback int) int {
	if value > 0 {
		return value
	}
	return fallback
}
