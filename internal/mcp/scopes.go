package mcp

import (
	"encoding/json"
	"strings"
)

const (
	ScopeBlogRead      = "blog.read"
	ScopeCategoryRead  = "blog.category.read"
	ScopeDraftWrite    = "blog.draft.write"
	ScopePublish       = "blog.publish"
	ScopeUpload        = "blog.upload"
	ScopeCategoryWrite = "blog.category.write"
)

func normalizeScopes(scopes []string) []string {
	result := make([]string, 0, len(scopes))
	seen := make(map[string]struct{}, len(scopes))
	for _, scope := range scopes {
		scope = strings.TrimSpace(scope)
		if scope == "" {
			continue
		}
		if _, ok := seen[scope]; ok {
			continue
		}
		seen[scope] = struct{}{}
		result = append(result, scope)
	}
	return result
}

func parseScopes(value string) []string {
	value = strings.TrimSpace(value)
	if value == "" {
		return nil
	}
	if strings.HasPrefix(value, "[") {
		var scopes []string
		_ = json.Unmarshal([]byte(value), &scopes)
		return normalizeScopes(scopes)
	}
	return normalizeScopes(strings.Split(value, ","))
}

func hasScope(scopes []string, expected string) bool {
	for _, scope := range scopes {
		if scope == expected {
			return true
		}
	}
	return false
}

func containsString(values []string, expected string) bool {
	for _, value := range values {
		if value == expected {
			return true
		}
	}
	return false
}
