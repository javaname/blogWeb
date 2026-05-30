package middleware

import (
	"net/http"

	"blogWeb/internal/service"

	"github.com/gin-gonic/gin"
)

func RequireCSRF(sessions *service.SessionManager) gin.HandlerFunc {
	return func(c *gin.Context) {
		sessionID, err := c.Cookie(service.AdminSessionCookieName)
		if err != nil || sessionID == "" {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"code": "auth_required", "message": "请先登录"})
			return
		}
		token := c.GetHeader("X-CSRF-Token")
		valid, err := sessions.ValidateCSRF(c.Request.Context(), sessionID, token)
		if err != nil {
			c.AbortWithStatusJSON(http.StatusInternalServerError, gin.H{"code": "internal_error", "message": "CSRF 校验失败"})
			return
		}
		if !valid {
			c.AbortWithStatusJSON(http.StatusForbidden, gin.H{"code": "csrf_invalid", "message": "CSRF token 无效"})
			return
		}
		c.Next()
	}
}
