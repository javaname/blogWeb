package middleware

import "github.com/gin-gonic/gin"

func SecurityHeaders() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Header(
			"Content-Security-Policy",
			"default-src 'self'; "+
				"base-uri 'self'; "+
				"connect-src 'self'; "+
				"img-src 'self' data: https:; "+
				"style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; "+
				"font-src 'self' data: https://fonts.gstatic.com; "+
				"script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com; "+
				"object-src 'none'; "+
				"frame-ancestors 'none'",
		)
		c.Header("X-Content-Type-Options", "nosniff")
		c.Header("Referrer-Policy", "strict-origin-when-cross-origin")
		c.Header("X-Frame-Options", "DENY")
		c.Next()
	}
}
