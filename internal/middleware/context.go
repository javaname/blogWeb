package middleware

import (
	"github.com/gin-gonic/gin"

	"blogWeb/internal/service"
)

const sessionContextKey = "admin_session_user"

func SessionUser(c *gin.Context) *service.SessionUser {
	value, ok := c.Get(sessionContextKey)
	if !ok {
		return nil
	}
	user, _ := value.(*service.SessionUser)
	return user
}

func SetSessionUser(c *gin.Context, user *service.SessionUser) {
	c.Set(sessionContextKey, user)
}
