package service

import (
	"bytes"
	"encoding/base64"
	"image"
	"image/color"
	"image/png"
	"strings"
	"testing"

	"blogWeb/config"
)

func newUploadService(t *testing.T) *UploadService {
	t.Helper()
	cfg := config.UploadConfig{
		Dir:     t.TempDir(),
		MaxSize: 1024 * 1024,
		AllowedTypes: []string{
			"image/jpeg",
			"image/png",
			"image/gif",
			"image/webp",
		},
		AllowSVG: false,
		Reencode: true,
	}
	return NewUploadService(cfg)
}

func pngBytes(t *testing.T) []byte {
	t.Helper()
	img := image.NewRGBA(image.Rect(0, 0, 1, 1))
	img.Set(0, 0, color.RGBA{R: 255, G: 1, B: 1, A: 255})
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		t.Fatalf("encode png: %v", err)
	}
	return buf.Bytes()
}

func TestUploadServiceStoreAcceptsPNGAndNormalizesPath(t *testing.T) {
	t.Parallel()

	service := newUploadService(t)
	result, err := service.Store(bytes.NewReader(pngBytes(t)), "test.png")
	if err != nil {
		t.Fatalf("store png: %v", err)
	}
	if !strings.HasPrefix(result.URL, "/uploads/") {
		t.Fatalf("unexpected upload URL: %s", result.URL)
	}
	if !strings.HasSuffix(result.Filename, ".png") {
		t.Fatalf("unexpected filename: %s", result.Filename)
	}
}

func TestUploadServiceRejectsInvalidMediaAndBase64(t *testing.T) {
	t.Parallel()

	service := newUploadService(t)
	if _, err := service.Store(strings.NewReader("<svg></svg>"), "x.svg"); err == nil {
		t.Fatalf("expected svg reject")
	}

	if _, err := service.StoreBase64("x.png", "bad-base64"); err == nil {
		t.Fatalf("expected base64 reject")
	}

	huge := base64.StdEncoding.EncodeToString(bytes.Repeat([]byte("a"), int(service.config.MaxSize)+1))
	if _, err := service.StoreBase64("huge.png", huge); err == nil {
		t.Fatalf("expected size reject")
	}
}
