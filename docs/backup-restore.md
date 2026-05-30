# 备份与恢复说明

## 备份范围

- SQLite 数据库文件
- Redis 持久化文件
- `public/uploads/` 上传目录
- `config.yaml`

## SQLite 备份

示例：

```powershell
sqlite3 data/blog.db ".backup data/blog-backup.db"
```

如果生产环境未安装 `sqlite3` CLI，可先停写服务后直接复制数据库文件。

## Redis

确认 Redis 已启用 RDB 或 AOF。

备份时至少保留：

- `dump.rdb`
- `appendonly.aof`（若启用）

## 上传目录

直接归档：

```powershell
Compress-Archive -Path public\\uploads -DestinationPath uploads-backup.zip
```

## 恢复步骤

1. 停止 Web 与 MCP 服务。
2. 恢复 SQLite 数据库文件。
3. 恢复 Redis 持久化文件并重启 Redis。
4. 恢复 `public/uploads/`。
5. 检查 `config.yaml` 与环境地址是否匹配。
6. 启动服务并执行健康检查。
