package service

import (
	"context"
	"crypto/tls"
	"fmt"
	"net"
	"net/smtp"
	"strings"
	"time"

	"blogWeb/config"
)

type RegistrationEmail struct {
	Email string
	Code  string
	TTL   time.Duration
}

type RegistrationEmailSender interface {
	SendRegistrationCode(ctx context.Context, email, code string, ttl time.Duration) error
}

type SMTPRegistrationEmailSender struct {
	config config.EmailConfig
}

func NewSMTPRegistrationEmailSender(cfg config.EmailConfig) *SMTPRegistrationEmailSender {
	return &SMTPRegistrationEmailSender{config: cfg}
}

func (s *SMTPRegistrationEmailSender) SendRegistrationCode(ctx context.Context, email, code string, ttl time.Duration) error {
	if strings.TrimSpace(s.config.SMTPHost) == "" || strings.TrimSpace(s.config.Username) == "" || strings.TrimSpace(s.config.Password) == "" {
		return NewAppError(500, "email_unavailable", "邮箱服务尚未配置")
	}
	from := strings.TrimSpace(s.config.From)
	if from == "" {
		from = strings.TrimSpace(s.config.Username)
	}
	port := s.config.SMTPPort
	if port == 0 {
		port = 465
	}
	addr := fmt.Sprintf("%s:%d", s.config.SMTPHost, port)
	auth := smtp.PlainAuth("", s.config.Username, s.config.Password, s.config.SMTPHost)
	subject := "博客注册验证码"
	body := fmt.Sprintf("你的注册验证码是：%s\n\n验证码 %d 分钟内有效。若非本人操作，请忽略本邮件。", code, int(ttl.Minutes()))
	message := strings.Join([]string{
		"From: " + from,
		"To: " + email,
		"Subject: " + subject,
		"MIME-Version: 1.0",
		"Content-Type: text/plain; charset=UTF-8",
		"",
		body,
	}, "\r\n")

	if port == 465 {
		dialer := &net.Dialer{Timeout: 10 * time.Second}
		conn, err := tls.DialWithDialer(dialer, "tcp", addr, &tls.Config{ServerName: s.config.SMTPHost, MinVersion: tls.VersionTLS12})
		if err != nil {
			return err
		}
		defer conn.Close()
		client, err := smtp.NewClient(conn, s.config.SMTPHost)
		if err != nil {
			return err
		}
		defer client.Close()
		if err := client.Auth(auth); err != nil {
			return err
		}
		if err := client.Mail(from); err != nil {
			return err
		}
		if err := client.Rcpt(email); err != nil {
			return err
		}
		writer, err := client.Data()
		if err != nil {
			return err
		}
		if _, err := writer.Write([]byte(message)); err != nil {
			_ = writer.Close()
			return err
		}
		if err := writer.Close(); err != nil {
			return err
		}
		return client.Quit()
	}

	done := make(chan error, 1)
	go func() {
		done <- smtp.SendMail(addr, auth, from, []string{email}, []byte(message))
	}()
	select {
	case <-ctx.Done():
		return ctx.Err()
	case err := <-done:
		return err
	}
}
