package service

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/url"
	"regexp"
	"strings"
	"time"
	"unicode"
)

var (
	slugPattern       = regexp.MustCompile(`^[a-z0-9]+(?:-[a-z0-9]+)*$`)
	coverImagePattern = regexp.MustCompile(`^/uploads/\d{4}/\d{2}/[a-zA-Z0-9_-]+\.(jpg|jpeg|png|gif|webp)$`)
)

func NewToken(length int) (string, error) {
	if length <= 0 {
		length = 32
	}
	buf := make([]byte, length)
	if _, err := rand.Read(buf); err != nil {
		return "", err
	}
	return hex.EncodeToString(buf), nil
}

func HashDigest(parts ...string) string {
	h := sha256.New()
	for _, part := range parts {
		_, _ = h.Write([]byte(part))
		_, _ = h.Write([]byte{0})
	}
	return "sha256:" + hex.EncodeToString(h.Sum(nil))
}

func Slugify(title string) string {
	title = strings.ToLower(strings.TrimSpace(title))
	if title == "" {
		return fmt.Sprintf("article-%d", time.Now().Unix())
	}

	var builder strings.Builder
	lastDash := false
	for _, r := range title {
		switch {
		case unicode.IsLetter(r) || unicode.IsDigit(r):
			if r > unicode.MaxASCII {
				continue
			}
			builder.WriteRune(r)
			lastDash = false
		case r == ' ' || r == '-' || r == '_' || r == '.' || r == '/':
			if !lastDash && builder.Len() > 0 {
				builder.WriteByte('-')
				lastDash = true
			}
		}
	}

	slug := strings.Trim(builder.String(), "-")
	if slug == "" {
		return fmt.Sprintf("article-%d", time.Now().Unix())
	}
	return slug
}

func IsValidSlug(slug string) bool {
	if len(slug) == 0 || len(slug) > 160 {
		return false
	}
	return slugPattern.MatchString(slug)
}

func CollapseSpaces(s string) string {
	return strings.Join(strings.Fields(s), " ")
}

func BuildExcerpt(content string, limit int) string {
	if limit <= 0 {
		limit = 200
	}

	replacer := strings.NewReplacer(
		"#", " ",
		"*", " ",
		"`", " ",
		">", " ",
		"[", " ",
		"]", " ",
		"(", " ",
		")", " ",
		"\n", " ",
		"\r", " ",
	)
	plain := CollapseSpaces(replacer.Replace(content))
	runes := []rune(plain)
	if len(runes) <= limit {
		return plain
	}
	return string(runes[:limit]) + "..."
}

func ValidateCoverImagePath(path string) bool {
	if path == "" {
		return true
	}
	if coverImagePattern.MatchString(path) {
		return true
	}
	parsed, err := url.Parse(path)
	if err != nil {
		return false
	}
	return parsed.Scheme == "https" && parsed.Host != ""
}

type Cursor struct {
	IsPinned    int       `json:"is_pinned"`
	PublishedAt time.Time `json:"published_at"`
	ID          uint      `json:"id"`
}

func EncodeCursor(cursor Cursor) (string, error) {
	data, err := json.Marshal(cursor)
	if err != nil {
		return "", err
	}
	return string(data), nil
}

func DecodeCursor(value string) (*Cursor, error) {
	if strings.TrimSpace(value) == "" {
		return nil, nil
	}
	var cursor Cursor
	if err := json.Unmarshal([]byte(value), &cursor); err != nil {
		return nil, NewAppError(400, "invalid_cursor", "无效的分页游标")
	}
	return &cursor, nil
}
