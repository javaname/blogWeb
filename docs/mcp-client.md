# MCP 客户端接入说明

## 支持的传输

- `stdio`
- HTTP `POST /mcp`

## Rust 启动命令

```powershell
cargo run -- db migrate --apply -config config.yaml
cargo run -- serve-mcp -transport stdio -config config.yaml
cargo run -- serve-mcp -transport http -config config.yaml
```

`serve-mcp` 只做 migration check，不会自动创建或迁移数据库。

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

- 使用 `cargo run -- mcp issue-token -config config.yaml -name <name> -scopes <scopes> -transport http` 签发
- 明文 token 只显示一次
- 数据库存储的是哈希，不是明文
- 使用 `cargo run -- mcp revoke-token -config config.yaml -name <name>` 立即撤销

## Rust 实现说明

- HTTP MCP 已支持 resources、tools、prompts、audit 和 read/write/publish/upload 分桶限流。
- stdio 默认只暴露只读能力；写能力需要显式设置 `mcp.stdio_write_enabled=true`。
- 当前 Rust 限流为进程内计数器；多进程共享限流需后续接 Redis。
- 上传 tool 会校验真实图片签名并保存原始图片字节，不做 Go 版 reencode。
