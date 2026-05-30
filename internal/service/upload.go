package service

import (
	"bytes"
	"encoding/base64"
	"fmt"
	"image"
	"image/gif"
	"image/jpeg"
	"image/png"
	"io"
	"mime/multipart"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"blogWeb/config"

	"github.com/google/uuid"
	"golang.org/x/image/webp"
)

type UploadService struct {
	config config.UploadConfig
}

type UploadResult struct {
	URL      string `json:"url"`
	Filename string `json:"filename"`
	MIMEType string `json:"mime_type,omitempty"`
	Size     int    `json:"size,omitempty"`
}

func NewUploadService(cfg config.UploadConfig) *UploadService {
	return &UploadService{config: cfg}
}

func (s *UploadService) StoreMultipart(fileHeader *multipart.FileHeader) (*UploadResult, error) {
	file, err := fileHeader.Open()
	if err != nil {
		return nil, err
	}
	defer file.Close()
	return s.Store(file, fileHeader.Filename)
}

func (s *UploadService) StoreBase64(filename, contentBase64 string) (*UploadResult, error) {
	data, err := base64.StdEncoding.DecodeString(contentBase64)
	if err != nil {
		return nil, NewAppError(400, "invalid_params", "content_base64 不是合法的 base64")
	}
	return s.Store(bytes.NewReader(data), filename)
}

func (s *UploadService) Store(reader io.Reader, filename string) (*UploadResult, error) {
	data, err := io.ReadAll(io.LimitReader(reader, s.config.MaxSize+1))
	if err != nil {
		return nil, err
	}
	if int64(len(data)) > s.config.MaxSize {
		return nil, NewAppError(413, "payload_too_large", "文件大小超过 5MB 限制")
	}

	contentType := http.DetectContentType(data)
	if !s.allowedType(contentType) {
		return nil, NewAppError(415, "unsupported_media_type", "不支持的文件类型，仅允许 jpg/png/gif/webp")
	}

	ext, normalizedType, output, err := s.normalizeImage(data, contentType)
	if err != nil {
		return nil, err
	}

	now := time.Now().UTC()
	dir := filepath.Join(s.config.Dir, fmt.Sprintf("%04d", now.Year()), fmt.Sprintf("%02d", int(now.Month())))
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return nil, err
	}

	name := uuid.NewString() + ext
	path := filepath.Join(dir, name)
	if err := os.WriteFile(path, output, 0o644); err != nil {
		return nil, err
	}

	return &UploadResult{
		URL:      filepath.ToSlash(filepath.Join("/uploads", fmt.Sprintf("%04d", now.Year()), fmt.Sprintf("%02d", int(now.Month())), name)),
		Filename: name,
		MIMEType: normalizedType,
		Size:     len(output),
	}, nil
}

func (s *UploadService) allowedType(contentType string) bool {
	normalized := strings.ToLower(strings.TrimSpace(strings.Split(contentType, ";")[0]))
	if normalized == "image/svg+xml" {
		return s.config.AllowSVG
	}
	for _, allowed := range s.config.AllowedTypes {
		if normalized == allowed {
			return true
		}
	}
	return false
}

func (s *UploadService) normalizeImage(data []byte, contentType string) (string, string, []byte, error) {
	normalized := strings.ToLower(strings.TrimSpace(strings.Split(contentType, ";")[0]))
	switch normalized {
	case "image/jpeg":
		if !s.config.Reencode {
			return ".jpg", normalized, data, nil
		}
		img, err := jpeg.Decode(bytes.NewReader(data))
		if err != nil {
			return "", "", nil, NewAppError(400, "unsupported_media_type", "JPEG 文件无效")
		}
		var buf bytes.Buffer
		if err := jpeg.Encode(&buf, img, &jpeg.Options{Quality: 90}); err != nil {
			return "", "", nil, err
		}
		return ".jpg", normalized, buf.Bytes(), nil
	case "image/png":
		if !s.config.Reencode {
			return ".png", normalized, data, nil
		}
		img, err := png.Decode(bytes.NewReader(data))
		if err != nil {
			return "", "", nil, NewAppError(400, "unsupported_media_type", "PNG 文件无效")
		}
		var buf bytes.Buffer
		if err := png.Encode(&buf, img); err != nil {
			return "", "", nil, err
		}
		return ".png", normalized, buf.Bytes(), nil
	case "image/gif":
		if !s.config.Reencode {
			return ".gif", normalized, data, nil
		}
		img, err := gif.Decode(bytes.NewReader(data))
		if err != nil {
			return "", "", nil, NewAppError(400, "unsupported_media_type", "GIF 文件无效")
		}
		var buf bytes.Buffer
		if err := gif.Encode(&buf, img, nil); err != nil {
			return "", "", nil, err
		}
		return ".gif", normalized, buf.Bytes(), nil
	case "image/webp":
		if _, err := webp.Decode(bytes.NewReader(data)); err != nil {
			return "", "", nil, NewAppError(400, "unsupported_media_type", "WEBP 文件无效")
		}
		return ".webp", normalized, data, nil
	default:
		_, _, err := image.DecodeConfig(bytes.NewReader(data))
		if err != nil {
			return "", "", nil, NewAppError(400, "unsupported_media_type", "图片文件无效")
		}
		return "", "", nil, NewAppError(415, "unsupported_media_type", "不支持的文件类型，仅允许 jpg/png/gif/webp")
	}
}
