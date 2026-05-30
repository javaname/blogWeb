package mcp

func (s *Server) resourceTemplates(includeWrites bool) []map[string]any {
	templates := []map[string]any{
		{"uri": "blog://site/meta", "name": "site_meta"},
		{"uri": "blog://categories", "name": "categories"},
		{"uriTemplate": "blog://articles/{slug}", "name": "article_by_slug"},
		{"uriTemplate": "blog://categories/{slug}/articles", "name": "category_articles"},
	}
	if includeWrites {
		templates = append(templates, map[string]any{"uriTemplate": "blog://drafts/{id}", "name": "draft_by_id"})
	}
	return templates
}
