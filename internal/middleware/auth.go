package middleware

import (
	"net/http"

	"blogWeb/internal/service"

	"github.com/gin-gonic/gin"
)

func RequireAuth(auth *service.AuthService) gin.HandlerFunc {
	return func(c *gin.Context) {
		sessionID, err := c.Cookie(service.AdminSessionCookieName)
		if err != nil || sessionID == "" {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"code": "auth_required", "message": "请先登录"})
			return
		}
		user, err := auth.CurrentUser(c.Request.Context(), sessionID)
		if err != nil {
			c.AbortWithStatusJSON(http.StatusInternalServerError, gin.H{"code": "internal_error", "message": "会话校验失败"})
			return
		}
		if user == nil {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"code": "auth_required", "message": "请先登录"})
			return
		}
		SetSessionUser(c, user)
		c.Next()
	}
}

func RequireAdmin() gin.HandlerFunc {
	return func(c *gin.Context) {
		user := SessionUser(c)
		if user == nil {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"code": "auth_required", "message": "请先登录"})
			return
		}
		if user.Role != "admin" {
			c.AbortWithStatusJSON(http.StatusForbidden, gin.H{"code": "forbidden", "message": "无权限访问"})
			return
		}
		c.Next()
	}
}
