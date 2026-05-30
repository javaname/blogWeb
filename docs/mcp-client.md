# MCP 客户端接入说明

## 支持的传输

- `stdio`
- HTTP `POST /mcp`

## HTTP 请求要求

- `Content-Type: application/json`
- `Accept` 包含 `application/json`
- `Authorization: Bearer <token>`
- `MCP-Protocol-Version` 使用配置允许的版本
- `Origin` 在白名单内

## stdio 注意事项

- `stdout` 仅输出 MCP 协议消息
- 日志和错误信息写入 `stderr`
- 默认只开放只读能力
- 若要允许写工具，需设置 `mcp.stdio_write_enabled=true`

## Token 管理

- 使用 `go run . mcp issue-token ...` 签发
- 明文 token 只显示一次
- 数据库存储的是哈希，不是明文
- 使用 `go run . mcp revoke-token ...` 立即撤销
