# 安全测试说明

## 已覆盖自动化场景

- Markdown/XSS 清洗
- CSRF 缺失拦截
- 登录爆破限流
- 点赞刷量限流
- SVG 上传拦截
- 伪装图片上传拦截
- 安全响应头生效
- 公开文章不泄漏草稿和未来发布时间文章

## 执行方式

```powershell
go test ./...
```

重点测试文件：

- `internal/service/renderer_test.go`
- `internal/service/upload_test.go`
- `internal/handler/http_test.go`
- `internal/mcp/http_test.go`
- `internal/mcp/server_test.go`
- `internal/mcp/transport_stdio_test.go`

## 人工补充检查

- 生产环境确认 HTTPS
- Redis 开启持久化
- 管理员初始密码已替换
- 上传目录权限只允许静态访问
- MCP HTTP 未直接公网暴露，或已配反向代理 ACL
