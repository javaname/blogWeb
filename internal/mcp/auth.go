package mcp

import "net/http"

type mcpError struct {
	Status  int
	Code    string
	Message string
	Scope   string
}

func (e *mcpError) Error() string {
	return e.Message
}

func (s *Server) writeAuthHeaders(w http.ResponseWriter, err *mcpError) {
	if err == nil {
		return
	}
	switch err.Status {
	case http.StatusUnauthorized:
		w.Header().Set("WWW-Authenticate", `Bearer resource_metadata="private-token-doc"`)
	case http.StatusForbidden:
		if err.Scope != "" {
			w.Header().Set("WWW-Authenticate", `Bearer error="insufficient_scope", scope="`+err.Scope+`"`)
		}
	}
}
