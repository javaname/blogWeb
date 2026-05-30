# blogWeb

个人博客系统，包含：

- Go 后端 Web 站点
- React 管理后台
- MCP Server（stdio / HTTP）

## 启动

### Web

```powershell
go run . serve-web -config config.yaml
```

默认命令也是 `serve-web`：

```powershell
go run .
```

### MCP

```powershell
go run . serve-mcp --transport=stdio -config config.yaml
go run . serve-mcp --transport=http -config config.yaml
```

### 前端

```powershell
cd client
npm run dev
```

## 测试

```powershell
go test ./...
go build ./...
```

## MCP Token

签发：

```powershell
go run . mcp issue-token --name reader --scopes blog.read --transport http
```

撤销：

```powershell
go run . mcp revoke-token --name reader
```

## 文档

- [实现任务清单](docs/superpowers/specs/2026-05-13-blog-implementation-tasks.md)
- [MCP 实施规格](docs/superpowers/specs/2026-05-14-blog-mcp-implementation-spec.md)
- [安全测试说明](docs/security-test.md)
- [备份恢复说明](docs/backup-restore.md)
- [MCP 安全测试说明](docs/mcp-security-test.md)
