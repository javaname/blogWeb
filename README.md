# blogWeb

个人博客系统，包含：

- Rust 后端 Web 站点
- React 管理后台
- MCP Server（stdio / HTTP）

## 启动

### Web

```powershell
cargo run -- db migrate --apply -config config.yaml
cargo run -- serve-web -config config.yaml
```

默认命令也是 `serve-web`：

```powershell
cargo run -- -config config.yaml
```

### MCP

```powershell
cargo run -- serve-mcp -transport stdio -config config.yaml
cargo run -- serve-mcp -transport http -config config.yaml
```

`serve-web` 和 `serve-mcp` 启动时只检查迁移状态，不会隐式创建数据库、执行迁移或写入 seed 数据。新库先运行 `db migrate --dry-run` / `db migrate --apply`。

### 前端

```powershell
cd client
npm run dev
```

## 测试

```powershell
cargo test --offline
npm --prefix client run check:i18n
npm --prefix client run check:ui
npm --prefix client run build
```

## MCP Token

签发：

```powershell
cargo run -- mcp issue-token -config config.yaml -name reader -scopes blog.read -transport http
```

撤销：

```powershell
cargo run -- mcp revoke-token -config config.yaml -name reader
```

## 文档

- [实现任务清单](docs/superpowers/specs/2026-05-13-blog-implementation-tasks.md)
- [MCP 实施规格](docs/superpowers/specs/2026-05-14-blog-mcp-implementation-spec.md)
- [安全测试说明](docs/security-test.md)
- [备份恢复说明](docs/backup-restore.md)
- [MCP 安全测试说明](docs/mcp-security-test.md)
