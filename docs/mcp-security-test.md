# MCP 安全测试说明

## 自动化覆盖

- 缺少 token 返回 `401`
- 无效 token / 已撤销 token 被拒绝
- scope 不足返回 `403` 且带 `WWW-Authenticate`
- 非法 Origin 拒绝
- 非法 `Accept` / `MCP-Protocol-Version` 拒绝
- `cover_image` 外部 URL / 路径穿越拒绝
- `preview_markdown` 不泄漏脚本
- `upload_image` 伪装图片 / 超大 payload 拒绝
- HTTP 读/上传限流生效
- stdio 默认不暴露写工具和草稿资源

## 执行方式

```powershell
go test ./internal/mcp/...
```

## 人工核对

- `mcp.stdio_write_enabled=false` 时，本地客户端只能看到只读能力
- `mcp.http_enabled=false` 默认不对公网开放
- 仅为需要的客户端签发最小 scope token
- 检查 `mcp_audit_logs` 中无明文 token、正文、base64 图片内容
