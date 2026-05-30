package mcp

import (
	"encoding/base64"
	"strings"
	"time"

	"blogWeb/internal/service"
)

func validateSlug(slug string) error {
	if !service.IsValidSlug(slug) {
		return service.NewAppError(400, "invalid_params", "slug 不合法")
	}
	return nil
}

func validateTitle(title string) error {
	title = strings.TrimSpace(title)
	if len([]rune(title)) == 0 || len([]rune(title)) > 120 {
		return service.NewAppError(400, "invalid_params", "title 长度需为 1-120 字符")
	}
	return nil
}

func validateMarkdown(content string) error {
	if strings.TrimSpace(content) == "" {
		return service.NewAppError(400, "invalid_params", "content 不能为空")
	}
	if len([]rune(content)) > 200000 {
		return service.NewAppError(400, "invalid_params", "content 超出最大长度")
	}
	return nil
}

func validateCategoryName(name string) error {
	name = strings.TrimSpace(name)
	if len([]rune(name)) == 0 || len([]rune(name)) > 40 {
		return service.NewAppError(400, "invalid_params", "分类名称长度需为 1-40 字符")
	}
	return nil
}

func validateCoverImage(path string) error {
	if !service.ValidateCoverImagePath(path) {
		return service.NewAppError(400, "invalid_params", "cover_image 只能引用站内上传路径或 https 图片")
	}
	return nil
}

func validateBase64Size(contentBase64 string, limit int64) error {
	data, err := base64.StdEncoding.DecodeString(contentBase64)
	if err != nil {
		return service.NewAppError(400, "invalid_params", "content_base64 不是合法的 base64")
	}
	if int64(len(data)) > limit {
		return service.NewAppError(413, "payload_too_large", "上传内容超过大小限制")
	}
	return nil
}

func parseRFC3339(value string) (*time.Time, error) {
	if strings.TrimSpace(value) == "" {
		return nil, nil
	}
	parsed, err := time.Parse(time.RFC3339, value)
	if err != nil {
		return nil, service.NewAppError(400, "invalid_params", "时间格式必须为 RFC3339")
	}
	return &parsed, nil
}
