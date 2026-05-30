# Rust 后端一次性替换设计

## 背景

当前项目后端使用 Go 实现，包含博客前台、React 管理后台 API、SQLite 数据访问、Redis 会话与限流、图片上传、邮箱验证码注册、MCP stdio/HTTP 服务和 MCP token 管理命令。用户选择“一次性替换”策略：Rust 后端完成并验证后，生产路径不再依赖 Go 服务。

## 目标

Rust 后端完整接管现有后端能力，并保持外部行为兼容：

- 保持 `config.yaml` 配置结构兼容。
- 复用 `migrations/*.sql`，不重新定义现有 SQLite 数据含义。
- 保持前端和模板已使用的 HTTP 路由、请求体、响应字段、状态码和错误码。
- 保持命令能力：`serve-web`、`serve-mcp --transport=stdio|http`、`mcp issue-token`、`mcp revoke-token`。
- 保持上传文件目录、静态资源目录、模板目录和管理后台资源路径兼容。
- 保持 MCP 客户端、审计日志、授权 scope、HTTP origin 校验和限流行为兼容。

## 非目标

- 不重设计前端页面。
- 不更换数据库类型。
- 不做数据迁移格式升级。
- 不引入读者账号体系之外的新业务能力。
- 不把真实邮箱账号、授权码或 token 写入仓库。

## 技术栈

- Web 框架：`axum`
- 异步运行时：`tokio`
- SQLite：`sqlx`，使用动态查询，直接兼容现有 schema
- Redis：`redis` async client
- 配置：`serde` + `serde_yaml`
- CLI：`clap`
- 模板：`tera`，运行时加载 `templates/*.html`
- Markdown：`pulldown-cmark`
- HTML 清理：`ammonia`
- 密码哈希：`bcrypt`
- 邮件发送：`lettre`
- 图片处理：`image`
- UUID：`uuid`
- 错误处理：`thiserror` + 统一 API error envelope
- 测试：Rust integration tests + HTTP router tests

## 目录结构

```text
Cargo.toml
src/
  main.rs
  app.rs
  config.rs
  db.rs
  error.rs
  http/
    mod.rs
    public.rs
    admin.rs
    auth.rs
    upload.rs
    presentation.rs
  middleware/
    mod.rs
    auth.rs
    csrf.rs
    security.rs
    anonymous.rs
  model/
    mod.rs
  service/
    mod.rs
    article.rs
    auth.rs
    category.rs
    comment.rs
    email.rs
    like.rs
    rate_limiter.rs
    renderer.rs
    session.rs
    upload.rs
  mcp/
    mod.rs
    auth.rs
    server.rs
    http.rs
    stdio.rs
    tools.rs
    resources.rs
    prompts.rs
tests/
  http_compat.rs
  service_compat.rs
  mcp_compat.rs
```

Go 代码在迁移期间保留为行为参考。Rust 验证通过后，再由单独变更删除或归档 Go 文件，避免迁移中丢失对照实现。

## 应用装配

Rust `AppState` 对齐 Go `Application`：

- `Config`
- SQLite pool
- Redis client
- Tera templates
- Article service
- Category service
- Comment service
- Like service
- Auth service
- Session service
- Rate limiter
- Upload service
- MCP server state

启动流程：

1. 解析 CLI。
2. 加载 `config.yaml`。
3. 创建数据库目录和上传目录。
4. 打开 SQLite pool。
5. 执行 migration `check`：只验证 schema，不写数据库；缺迁移或缺 `schema_migrations` 时拒绝启动，并提示运行 `db migrate --dry-run` / `db migrate --apply`。
6. 创建 Redis client；`serve-web` 和 `serve-mcp` 默认要求 Redis 可用，token 管理命令允许只操作 SQLite。
7. 按初始化数据策略创建缺失管理员；演示内容只在 `seed.demo_content_enabled=true` 时写入。
8. 根据命令启动 Web、MCP stdio、MCP HTTP 或执行 token 管理。

### 迁移安全策略

Rust 版不能盲目重跑 SQL 后忽略错误。迁移 runner 规则如下：

- 提供三种模式：`check`、`dry-run`、`apply`。`check` 只检查 schema；`dry-run` 在数据库副本上执行迁移；`apply` 才修改目标库。
- 命令契约：`cargo run -- db check -config config.yaml`、`cargo run -- db migrate --dry-run -config config.yaml`、`cargo run -- db migrate --apply -config config.yaml`。
- `serve-web` 和 `serve-mcp` 默认只允许已完成 schema 通过 `check`；`check` 不写目标库，包括不创建 `schema_migrations`。
- 如需启动时自动 `apply`，必须显式配置 `database.auto_migrate: true`；生产配置默认不得开启。
- 启动前如果 `database.path` 已存在且进入 `apply`，先在同目录生成备份：`blog.db.backup-YYYYMMDDHHMMSS`。测试环境可跳过备份。
- 启动事务，执行前记录 `PRAGMA user_version`、现有表列表和关键列列表。
- 使用 `schema_migrations` 表记录已应用文件：`version`、`filename`、`sha256`、`applied_at`。Go 版没有该表时，Rust 首次启动要按当前 schema 反推已应用迁移并补写记录。
- `CREATE TABLE IF NOT EXISTS` 和 `CREATE INDEX IF NOT EXISTS` 可直接执行。
- `ALTER TABLE ... ADD COLUMN` 先用 `PRAGMA table_info(table)` 检查列是否存在；已存在则跳过，不依赖错误字符串。
- migration 文件 hash 变更时拒绝启动，提示先人工确认。
- 任一 migration 失败时回滚事务，不启动服务，并保留备份。
- 迁移完成后执行 schema smoke check，确认目标表和关键列存在。

首次接管判定：

- 如果 `schema_migrations` 不存在但所有目标表和关键列均存在，`check` 返回“可接管但未登记”并失败；只有 `db migrate --apply` 可以补写 `001` 到 `005` 的记录，hash 使用当前仓库 migration 文件 hash。
- 如果部分关键列缺失且表中无业务数据，允许在 `apply` 模式补迁移。
- 如果部分关键列缺失且相关表已有业务数据，`check` 和 `serve-*` 必须失败，提示先备份并运行 `db migrate --dry-run`；`apply` 必须创建备份后执行。
- 如果目标表缺失但数据库非空，禁止自动推断，要求显式运行 `db migrate --apply`。
- 如果 migration 文件 hash 与 `schema_migrations` 已记录值不一致，拒绝启动和迁移，除非后续新增专门的 `db accept-migration-hash` 命令；本次迁移不实现该命令。

### 初始化数据策略

管理员初始化和演示内容必须保持 Go 版幂等行为，同时避免污染生产库：

- 新增配置 `seed.demo_content_enabled`，默认 `false`。只有显式为 `true` 时才写入演示用户、分类和文章。
- `EnsureInitialAdmin` 只在 `users.username = admin.init_username` 不存在时创建，不覆盖已有密码、邮箱或角色。
- 如果 `admin.init_password` 等于 `change-me-123456` 或 `replace-with-secure-password`，且目标管理员不存在，生产启动必须失败；测试和显式 `seed.allow_insecure_admin_password=true` 例外。
- 演示内容仅在 `seed.demo_content_enabled=true` 且 `articles` 表中 `status = 'published'` 的数量为 0 时写入。
- 演示用户和分类按唯一字段存在则跳过。
- 若配置未包含 `seed` 节，按默认值处理，不影响旧 `config.yaml` 解析。

## HTTP 兼容范围

公共页面：

- `GET /healthz`
- `GET /`
- `GET /articles/:slug`
- `GET /categories/:slug`
- `GET /admin`
- `GET /admin/*filepath`
- `GET /uploads/*path`
- `GET /assets/*path`

公共 API：

- `GET /api/articles`
- `GET /api/articles/:slug`
- `POST /api/articles/:slug/like`
- `POST /api/articles/:slug/bookmark`
- `POST /api/articles/:slug/comments`
- `POST /api/authors/:id/follow`
- `POST /api/newsletter/subscribe`
- `POST /api/likes/batch`
- `POST /api/auth/register/code`
- `POST /api/auth/register`

后台 API：

- `POST /api/admin/login`
- `POST /api/admin/logout`
- `GET /api/admin/csrf-token`
- `GET /api/admin/me`
- `GET /api/admin/dashboard`
- `GET /api/admin/settings`
- `PUT /api/admin/settings`
- `GET /api/admin/articles`
- `POST /api/admin/articles`
- `GET /api/admin/articles/:id`
- `PUT /api/admin/articles/:id`
- `DELETE /api/admin/articles/:id`
- `GET /api/admin/categories`
- `POST /api/admin/categories`
- `PUT /api/admin/categories/:id`
- `DELETE /api/admin/categories/:id`
- `PUT /api/admin/categories/sort`
- `GET /api/admin/comments`
- `PUT /api/admin/comments/:id/status`
- `DELETE /api/admin/comments/:id`
- `POST /api/admin/upload`

所有后台写操作继续要求管理员 session 和 CSRF token。

### HTTP 兼容契约

所有 JSON API 错误响应保持当前 envelope：

```json
{ "code": "invalid_params", "message": "请求体格式错误" }
```

非 `AppError` 映射为 `500 {"code":"internal_error","message":"服务端错误"}`。认证中间件使用固定响应：

- 未登录：`401 {"code":"auth_required","message":"请先登录"}`
- 非管理员：`403 {"code":"forbidden","message":"无权限访问"}`
- CSRF 无效：`403 {"code":"csrf_invalid","message":"CSRF token 无效"}`

公共约定：

- JSON 请求必须接受 `Content-Type: application/json`；multipart 上传例外。
- 前端 fetch 使用 `credentials: include`。
- CSRF header 名称为 `X-CSRF-Token`。
- 匿名读者状态读取 `X-Anonymous-Id` header；页面访问时若缺少 `anonymous_id` cookie，则设置 `anonymous_id=<random>; Max-Age=31536000; Path=/; HttpOnly`。
- `clientIP` 按 Go 版行为优先取请求远端地址，不从代理头推断，除非后续单独设计反向代理信任策略。
- 安全响应头必须一致：`Content-Security-Policy`、`X-Content-Type-Options: nosniff`、`Referrer-Policy: strict-origin-when-cross-origin`、`X-Frame-Options: DENY`。

核心接口矩阵：

| 路由 | Query/Body | 成功响应 | 兼容重点 |
|---|---|---|---|
| `GET /healthz` | 无 | `200 {"status":"ok"}` | 无 Redis 业务读写 |
| `GET /api/articles` | `limit` 默认 12 最大 50，`cursor`，`category`，`keyword` | `200 ListPublishedResult` | 只返回已发布且发布时间不晚于当前 UTC；排序 `is_pinned DESC, published_at DESC, id DESC` |
| `GET /api/articles/:slug` | path slug | `200 PublicArticleDetail` 或 `301` | 历史 slug 命中时永久重定向到新 slug |
| `POST /api/articles/:slug/like` | `{"action":"like"|"unlike"}`，`X-Anonymous-Id` | `200 {"liked":bool,"like_count":int}` | 缺匿名 ID 返回 `400 invalid_params`；按 IP 和文章维度限流 |
| `POST /api/articles/:slug/bookmark` | `{"action":"bookmark"|"unbookmark"}`，`X-Anonymous-Id` | `200 {"bookmarked":bool}` | 按文章和匿名 ID 唯一 |
| `POST /api/authors/:id/follow` | `{"action":"follow"|"unfollow"}`，`X-Anonymous-Id` | `200 {"followed":bool}` | 作者 ID 必须存在 |
| `POST /api/likes/batch` | `{"article_slugs":["..."]}`，`X-Anonymous-Id` | `200 {"liked_map":{slug:bool},"bookmarked_map":{slug:bool},"followed_author_map":{id:bool}}` | 前端依赖 map 字段存在 |
| `POST /api/articles/:slug/comments` | `{"author_name":"","content":"","parent_id":null}`，`X-Anonymous-Id` | `201 {"id":id,"parent_id":...,"status":"approved","message":"评论已发布"}` | 默认昵称 `匿名读者`；敏感词返回 `400 comment_policy_violation` |
| `POST /api/newsletter/subscribe` | `{"email":"..."}`，`X-Anonymous-Id` | `201/200 {"subscribed":true}` | 邮箱唯一，重复订阅保持成功语义 |
| `POST /api/auth/register/code` | `{"email":"reader@example.com"}` | `201 {"sent":true,"expires_in":600}` | 邮箱小写去空格；fake sender 用于测试 |
| `POST /api/auth/register` | `{"email":"","code":"","password":"","confirm_password":""}` | `201 {"user":{...}}` | 成功后 `role=user`，验证码标记 used |
| `POST /api/admin/login` | `{"username":"","password":""}` | `200 {"user":{...}}` + `admin_session` cookie | 用户名或邮箱均可登录 |
| `POST /api/admin/logout` | `{}` | `200 {"ok":true}` + 清 cookie | 清 Redis `session:*` 和 `csrf:*` |
| `GET /api/admin/csrf-token` | cookie | `200 {"csrf_token":"..."}` | 调用会刷新 session last_seen |
| `GET /api/admin/articles` | `page` 默认 1，`page_size` 默认 20 最大 100，`status`，`keyword` | `200 {"list":[],"page":...,"page_size":...,"total":...}` | 管理端包含草稿 |
| `POST/PUT /api/admin/articles` | article editor JSON，见字段契约 | `201/200 ArticleEditorDetail` | 写操作要求 CSRF |
| `DELETE /api/admin/articles/:id` | path id | `200 {"deleted":true}` | 写操作要求 CSRF |
| `GET/POST/PUT/DELETE /api/admin/categories` | category JSON，见字段契约 | 见字段契约 | 分类名/slug 冲突为 `409 conflict` |
| `GET /api/admin/comments` | `page`、`page_size`、`status`、`keyword` | `200 CommentListResult` | `page_size` 默认 20 最大 100 |
| `PUT /api/admin/comments/:id/status` | `{"status":"approved|rejected|pending","rejection_reason":""}` | `200 comment` | 拒绝默认原因为 `不符合评论规范` |
| `POST /api/admin/upload` | multipart field `file` | `200 UploadResult` | SVG 默认拒绝；伪装图片拒绝 |
| `GET/PUT /api/admin/settings` | site settings JSON | `200 settings` | `site_settings` 持久化，同时保留 config fallback |

字段契约：

- `PublicArticleSummary`: `id:uint`、`title:string`、`slug:string`、`cover_image:string`、`excerpt:string`、`category?:PublicCategory|null`、`author?:PublicAuthor|null`、`is_pinned:bool`、`like_count:int`、`read_time_min:int`、`published_at:string|null`。
- `PublicCategory`: `id:uint`、`name:string`、`slug:string`。
- `PublicAuthor`: `id:uint`、`username:string`。
- `PublicArticleDetail`: summary 字段加 `content_html:string`、`user_liked:bool`、`user_bookmarked:bool`、`author_followed:bool`、`created_at:string`、`updated_at:string`。
- `ListPublishedResult`: `list:PublicArticleSummary[]`、`next_cursor:string`、`has_more:bool`。无下一页时 `next_cursor=""`。
- `AdminArticleSummary`: `id`、`title`、`slug`、`cover_image`、`status`、`is_pinned`、`category?:{id,name}`、`author?:{id,username}`、`like_count`、`published_at`、`created_at`、`updated_at`。
- `ListAdminResult`: `list:AdminArticleSummary[]`、`page:int`、`page_size:int`、`total:int`。
- `ArticleEditorDetail`: `id`、`title`、`slug`、`content`、`cover_image`、`category_id:uint|null`、`status`、`is_pinned`、`published_at:string|null`、`created_at`、`updated_at`。
- `CreateArticleInput`: `title:string`、`content:string`、`cover_image:string`、`category_id:uint|null`、`status:string`、`is_pinned:bool`、`published_at:string|null`。`author_id` 来自 session，不接受客户端覆盖。
- `UpdateArticleInput`: 所有字段可选；`category_id:null` 表示清空分类；`published_at:null` 表示清空发布时间。
- `PublicComment`: `id:uint`、`parent_id?:uint`、`author_name:string`、`content:string`、`relative_time:string`、`created_at:string`、`replies?:PublicComment[]`。
- `AdminComment`: `id:uint`、`article_id:uint`、`parent_id?:uint`、`article_title:string`、`author_name:string`、`content:string`、`status:string`、`rejection_reason:string`、`created_at:string`、`updated_at:string`。
- `CommentListResult`: `list:AdminComment[]`、`page:int`、`page_size:int`、`total:int`。
- `UploadResult`: `url:string`、`filename:string`、`mime_type?:string`、`size?:int`。
- `Dashboard`: `stats` 含 `total_articles`、`published_articles`、`draft_articles`、`total_comments`、`pending_comments`、`total_likes`、`monthly_views`、`followers`；`activity` 为 `{type,title,description,tone,icon,created_at}[]`；`views_trend` 为 `{date,views}[]`。
- `Settings`: `site:{title,description,base_url}`、`upload:{max_size,allowed_types,allow_svg,reencode}`、`publishing:{default_author,scheduled_publishing,pinned_stories}`、`mcp:{enabled,stdio_enabled,stdio_write_enabled,http_enabled,http_addr,http_path,require_origin_check,allowed_origins}`。
- `CategoryWithCount`: `id`、`name`、`slug`、`sort_order`、`created_at`、`article_count`。

如果字段契约和 golden 文件出现冲突，以 golden 文件为准；设计文档需要同步修正。

静态资源与 SPA fallback：

- `/admin` 和 `/admin/*filepath` 优先返回 `public/admin/index.html` 或真实 admin 静态文件；不存在路径回退到 admin index。
- `/uploads/*path` 从 `upload.dir` 读取，不允许路径穿越。
- `/assets/*path` 从 `public/assets` 读取，不允许路径穿越。

## 数据兼容

Rust 版直接读写现有表：

- `users`
- `categories`
- `articles`
- `likes`
- `slug_history`
- `mcp_clients`
- `mcp_audit_logs`
- `comments`
- `bookmarks`
- `author_follows`
- `newsletter_subscriptions`
- `site_settings`
- `email_verification_codes`

时间字段以 UTC 写入。SQLite bool 字段继续使用 `0/1`。唯一约束冲突继续映射为 `409 conflict`。不存在资源继续映射为 `404 not_found`。

## 认证、会话和限流

管理员登录继续使用 bcrypt 验证密码。Session 存在 Redis，cookie 名、过期时间、idle timeout 和 CSRF token 行为保持兼容。

Session 细节：

- cookie 名称：`admin_session`
- cookie 值：随机 token，生成自 24 字节随机数，使用 URL-safe base64 或等价编码；不得包含空白和分号
- 登录设置 cookie：`Path=/`、`Max-Age=session.max_age`、`HttpOnly=true`、`Secure=false`、`SameSite` 按 axum 默认或显式 `Lax`，golden 中记录最终值
- 登出删除 cookie：同名同 path，`Max-Age=-1` 或等价过期时间，`HttpOnly=true`
- Redis session key：`session:{session_id}`
- Redis csrf key：`csrf:{session_id}`
- Redis value：JSON，字段为 `user_id`、`username`、`role`、`csrf_token`、`created_at`、`last_seen`
- TTL：`session.max_age` 秒，每次保存 session 同步刷新 session key 和 csrf key TTL
- 绝对过期：`now - created_at > max_age` 时销毁
- 空闲过期：`now - last_seen > idle_timeout` 时销毁
- 每次 `Get` 成功后更新 `last_seen` 并保存
- CSRF 校验只接受 `X-CSRF-Token` header

限流继续使用 Redis `INCR` + 首次 `EXPIRE` 计数键，键名保持 Go 版本格式：

- `login_rate:{ip}`
- `login_fail:{username}`
- `registration_rate:{ip}`
- `registration_email_rate:{email}`
- `like_rate:{ip}`
- `like_article_rate:{ip}:{article_id}`
- `comment_rate:{ip}`
- `comment_article_rate:{ip}:{article_id}`
- `mcp_read_rate:{client_id}`
- `mcp_write_rate:{client_id}`
- `mcp_upload_rate:{client_id}`

配置值小于等于 0 时使用 Go 版默认值：登录 IP 600/20，登录用户失败 5/900，注册 IP 600/5，注册邮箱 600/3，点赞 IP 60/60，点赞文章 600/20，评论 IP 60/10，评论文章 600/5。

HTTP 超限响应：`429 {"code":"rate_limited","message":"请求过于频繁，请稍后再试"}`；当前 Go 版不设置 `Retry-After`，Rust 版也不设置，除非后续单独变更。MCP 限流默认值：read 120/min、write 30/min、publish 10/10min、upload 10/10min；超限返回 JSON-RPC error，HTTP status 429，data.code 为 `rate_limited`。

## 内容渲染和安全

Markdown 渲染使用 `pulldown-cmark`，再用 `ammonia` 清理 HTML。文章封面只允许站内 `/uploads/`、`/assets/` 或空值。上传继续限制 MIME、大小和 SVG 开关，默认重编码 jpg/png/gif，webp 校验后保留原内容。

安全响应头、匿名访客 cookie、管理员 CSRF、MCP token hash、MCP origin check 都必须纳入兼容测试。

## MCP 兼容范围

Rust MCP 模块需要覆盖：

- stdio JSON-RPC 传输
- HTTP JSON-RPC 传输
- initialize、resources、resource templates、prompts、tools
- 只读能力默认可用
- 写能力受 `stdio_write_enabled`、client scope 和 transport 控制
- token 签发与撤销
- 审计日志
- MCP read/write/upload 维度限流

MCP 协议实现优先保持当前项目行为兼容，不引入额外框架抽象，避免和现有客户端响应格式漂移。

### MCP JSON-RPC 契约

请求结构：

```json
{ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} }
```

响应结构：

```json
{ "jsonrpc": "2.0", "id": 1, "result": {} }
```

错误结构：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": 403,
    "message": "MCP token 缺少 blog.publish 权限",
    "data": {
      "code": "forbidden_scope",
      "message": "MCP token 缺少 blog.publish 权限",
      "request_id": "1"
    }
  }
}
```

HTTP 传输要求：

- path 必须等于 `mcp.http_path`，否则 404。
- `GET` 和非 `POST` 返回 405。
- `Content-Type` 必须包含 `application/json`。
- `Accept` 为空或包含 `application/json`。
- `MCP-Protocol-Version` 必须在 `mcp.protocol_versions` 中。
- `mcp.require_origin_check=true` 时，`Origin` 必须在 `mcp.allowed_origins` 中。
- 认证 header：`Authorization: Bearer <token>`。
- 401 设置 `WWW-Authenticate: Bearer resource_metadata="private-token-doc"`。
- scope 不足的 403 设置 `WWW-Authenticate: Bearer error="insufficient_scope", scope="<scope>"`。
- notification 请求 `id=null` 成功时返回 HTTP 202 且无 JSON body。

stdio 传输要求：

- 从 stdin 持续解码 JSON object，并逐条向 stdout 编码 JSON-RPC response。
- JSON 解码失败向 stderr 写错误，同时 stdout 返回合法 JSON-RPC error object：`{"jsonrpc":"2.0","error":{"code":400,"message":"JSON-RPC 请求格式错误"}}`；无法解析请求 id 时不包含 `id` 字段。
- `mcp.stdio_write_enabled=false` 时，所有写工具和 `blog://drafts/*` resource read 返回 `403 forbidden_scope`。

方法与 capability：

- `initialize` 返回 `serverInfo.name=blogWeb`、`serverInfo.version=v6`、`capabilities.resources.listChanged=false`、`capabilities.tools={}`、`capabilities.prompts={}`，并附带 `resources`。
- `resources/list` 返回 `{"resources":[...]}`。
- `resources/read` 参数 `{"uri":""}`。
- `tools/list` 返回 `{"tools":[{"name":"list_articles"},...]}`。
- `tools/call` 参数 `{"name":"","arguments":{...}}`。
- `prompts/list` 返回 `draft_article_from_outline`、`seo_review_article`、`rewrite_article_summary`。
- `prompts/get` 参数 `{"name":"","arguments":{...}}`。

resource templates：

- `{"uri":"blog://site/meta","name":"site_meta"}`
- `{"uri":"blog://categories","name":"categories"}`
- `{"uriTemplate":"blog://articles/{slug}","name":"article_by_slug"}`
- `{"uriTemplate":"blog://categories/{slug}/articles","name":"category_articles"}`
- 写能力开启时追加 `{"uriTemplate":"blog://drafts/{id}","name":"draft_by_id"}`

resource 输出：

- `blog://site/meta`: `title`、`description`、`base_url`、`version:"v6"`。
- `blog://categories`: `{"list": CategoryWithCount[]}`。
- `blog://articles/{slug}`: `id`、`title`、`slug`、`content_html`、`excerpt`、`category`、`is_pinned`、`published_at`、`updated_at`。
- `blog://drafts/{id}`: `ArticleEditorDetail`。
- `blog://categories/{slug}/articles`: `{"category":PublicCategory,"list":PublicArticleSummary[]}`。

tool schema：

- `list_articles`: arguments `cursor?:string`、`category?:string`、`limit?:int`，result `ListPublishedResult`。
- `get_article`: arguments `slug:string`，result `PublicArticleDetail`。
- `list_categories`: arguments `{}`，result `{"list": CategoryWithCount[]}`。
- `preview_markdown`: arguments `content:string`，result `{"content_html":string,"excerpt":string}`。
- `create_article_draft`: arguments `title:string`、`content:string`、`category_id?:uint|null`、`cover_image?:string`、`is_pinned?:bool`，result `{"id":uint,"slug":string,"status":"draft"}`。
- `update_article`: arguments `id:uint` plus optional `title`、`content`、`cover_image`、`category_id`、`is_pinned`，result `{"id":uint,"slug":string,"updated_at":string}`。
- `publish_article`: arguments `id:uint`、`published_at?:RFC3339 string`，result `{"id":uint,"status":"published","published_at":string|null}`。
- `unpublish_article`: arguments `id:uint`，result `{"id":uint,"status":"draft"}`。
- `upload_image`: arguments `filename:string`、`mime_type:string`、`content_base64:string`，result `UploadResult`。
- `create_category`: arguments `name:string`、`slug?:string`、`sort_order?:int`，result `{"id":uint,"name":string,"slug":string}`。
- `update_category`: arguments `id:uint` plus optional `name`、`slug`、`sort_order`，result `{"id":uint,"name":string,"slug":string,"sort_order":int}`。

prompt schema：

- `draft_article_from_outline`: arguments `title:string`、`outline:string`、`audience?:string`、`tone?:string`，result `{"name":name,"content":string,"input":arguments}`。
- `seo_review_article`: arguments `title:string`、`content:string`、`keywords?:string[]`，result 同上。
- `rewrite_article_summary`: arguments `title:string`、`content:string`、`target_length?:int`，result 同上。

scope 映射：

| 请求 | scope |
|---|---|
| `resources/read blog://site/meta` | `blog.read` |
| `resources/read blog://categories` | `blog.category.read` |
| `resources/read blog://articles/*` | `blog.read` |
| `resources/read blog://drafts/*` | `blog.draft.write` |
| `resources/read blog://categories/*/articles` | `blog.read` |
| `tools/call list_articles`、`get_article` | `blog.read` |
| `tools/call list_categories` | `blog.category.read` |
| `tools/call preview_markdown`、`create_article_draft`、`update_article` | `blog.draft.write` |
| `tools/call publish_article`、`unpublish_article` | `blog.publish` |
| `tools/call upload_image` | `blog.upload` |
| `tools/call create_category`、`update_category` | `blog.category.write` |

token 与审计：

- `mcp issue-token --name <name> --scopes <comma-separated> --transport <http|stdio|both> -config config.yaml`；缺 name/scopes 返回非零退出。
- 输出格式保持：`name=<name>`、`transport=<transport>`、`token=<token>` 分行打印。
- `mcp revoke-token --name <name> -config config.yaml` 将 `is_enabled=false`；目标不存在时保持幂等成功，除非数据库错误。
- token 明文只在签发命令输出一次。
- token hash 使用 `HMAC-SHA256(session.secret, token)` 的 hex 字符串。
- `session.secret` 变更会导致旧 token 失效；生产切换说明中必须提醒不要变更该 secret。
- scopes 以 JSON array 字符串存入 `mcp_clients.scopes`，同时兼容历史逗号分隔字符串读取。
- 审计字段保持：`client_id`、`transport`、`action_type`、`target`、`scope`、`status`、`request_id`、`actor_ip`、`error_code`、`payload_digest`、`created_at`。
- `payload_digest` 以当前 Go 行为为准；如果 Go 版写入原始 params 或 digest 命名不一致，golden baseline 记录后 Rust 照做。
- `request_id` 为 JSON-RPC `id` 的字符串表示；`id=null` 或缺失时为空字符串。
- HTTP `actor_ip` 保持 Go 行为：使用 `RemoteAddr`。

## 测试策略

严格按 TDD 推进。每个生产模块先写失败测试，再实现代码。

### Go 行为基线

进入 Rust 实现前先冻结 Go 行为，生成 golden baseline，避免把 Rust 错误行为写成新标准：

- 使用临时 SQLite fixture DB 和 miniredis 启动 Go test app。
- 对核心 HTTP 场景保存 status、headers、cookie、JSON body 到 `tests/golden/http/*.json`。
- 对核心 MCP JSON-RPC 场景保存 request/response/status/headers 到 `tests/golden/mcp/*.json`。
- 对 session 和限流保存 Redis key、TTL 范围和 value schema 断言，不保存随机 token 明文。
- 对模板和 Markdown 安全样例保存 snapshot，覆盖 `<script>`、链接、标题、摘要和中文格式化日期。
- Rust 测试读取 golden 文件，先断言兼容，再允许新增 Rust 内部单元测试。

Golden 归一化规则：

- 随机 token、session id、csrf token、MCP token 替换为占位符：`<SESSION_ID>`、`<CSRF_TOKEN>`、`<MCP_TOKEN>`。
- 自增 ID 在 fixture 固定插入顺序下保持确定；若由并发产生，归一化为 `<ID:n>` 映射。
- 时间字段按 RFC3339/SQLite 字符串解析后归一到 UTC；`created_at`、`updated_at` 可使用 fixture 固定时钟，不能固定时用 `<TIMESTAMP>` 占位。
- Cookie header 解析成结构化对象，比对 name、path、max_age/http_only/secure/same_site，忽略属性顺序。
- Header 比对只覆盖白名单：`Content-Type`、安全响应头、`Set-Cookie`、`Location`、`WWW-Authenticate`。忽略 `Date`、`Content-Length` 和 header 顺序。
- JSON body 以 canonical JSON 比对，忽略 object key 顺序，不忽略数组顺序。
- golden 更新必须通过显式命令 `UPDATE_GOLDEN=1 go test ./...` 或后续 Rust 等价命令，并在提交说明中解释差异。

第一批 Rust 测试：

- `cargo test` 在无 Rust 后端代码时失败，证明测试红线有效。
- 配置加载默认值和 YAML 覆盖。
- migration runner 能创建完整 schema。
- `GET /healthz` 返回 `200 {"status":"ok"}`。

HTTP 兼容测试：

- 公开文章列表隐藏草稿和未来发布时间。
- 历史 slug 返回 301。
- 点赞、收藏、关注、批量状态按匿名访客隔离。
- 评论创建、敏感词拒绝、后台审核。
- 管理员登录、session cookie、CSRF 拦截。
- 上传拒绝 SVG 和伪装图片。
- 邮箱验证码注册使用 fake sender，不连接真实 SMTP。

MCP 兼容测试：

- 未授权 HTTP MCP 请求被拒绝。
- scope 不足被拒绝。
- origin 校验生效。
- stdio 默认禁用写能力。
- 创建草稿、预览 Markdown、上传封面、发布流程可用。
- 审计日志写入。

## 实施顺序

一次性替换不等于一次性提交所有代码。实现按可运行切片推进，每个切片都必须能运行对应兼容测试：

1. 生成 Go golden baseline：HTTP、MCP、session/Redis、模板安全样例。
2. 写入 Cargo 工程和第一批失败测试。
3. 切片 A：配置、数据库迁移、应用装配、`/healthz`。
4. 切片 B：公开只读页面/API，包含模板渲染、文章列表、文章详情、分类页、静态资源。
5. 切片 C：管理员登录、session、CSRF、后台只读 API。
6. 切片 D：后台文章、分类、评论、设置写操作。
7. 切片 E：上传、Markdown 渲染、安全清理、封面路径验证。
8. 切片 F：读者互动、评论、订阅、邮箱验证码注册。
9. 切片 G：MCP read 能力、resources、prompts、tools/list。
10. 切片 H：MCP write 能力、token 签发/撤销、HTTP auth、审计、限流。
11. 更新 README 和运行脚本。
12. 运行 `cargo fmt`、`cargo test`、`cargo build`、前端构建和 Go baseline 对照测试。

每个切片完成条件：对应 golden 测试通过、无新增未处理 TODO、错误响应和安全 header 与 Go 基线一致。

## 风险与缓解

- 一次性替换范围大：通过兼容测试逐块锁定行为，Go 代码保留到 Rust 验证完成后再删除。
- SQLite 动态 schema 容易字段漂移：Rust 模型不重新声明迁移事实，测试直接跑现有 SQL。
- MCP 协议容易格式漂移：优先迁移当前测试覆盖的协议响应，再补关键客户端场景。
- Windows Rust 工具链可能依赖 MSVC 环境：在实施前确认 `cargo`、`rustc`、链接器可用。
- 邮箱注册当前计划状态与代码状态不完全一致：Rust 版以现有 migration、测试和设计文档作为首批兼容范围。
- 模板、Markdown 和 sanitizer 与 Go 库无法天然一致：用 golden snapshot 锁定关键输出；无法完全一致时优先保证安全和前端可用，并记录差异。
- 上传写文件失败可能留下部分文件：Rust 版先写临时文件，校验和重编码成功后原子重命名，失败清理临时文件。

## 生产切换和回滚

生产切换前必须执行：

1. 停止 Go 服务。
2. 备份 SQLite 数据库和 `public/uploads`。
3. 用 Rust 二进制对备份库做只读演练和 migration dry-run。
4. 启动 Rust 服务到临时端口，跑 health、登录、文章读取、MCP initialize smoke test。
5. 切换正式端口或服务脚本。
6. 保留 Go 二进制和原数据库备份，若 smoke test 失败，停止 Rust，恢复 Go 服务和数据库备份。

## 验收标准

- `cargo test` 通过。
- `cargo build` 通过。
- `cargo run -- serve-web -config config.yaml` 可启动 Web 服务。
- `cargo run -- serve-mcp --transport=stdio -config config.yaml` 可启动 stdio MCP。
- `cargo run -- serve-mcp --transport=http -config config.yaml` 可启动 HTTP MCP。
- `cargo run -- mcp issue-token ...` 和 `cargo run -- mcp revoke-token ...` 可用。
- 现有前端无需修改 API 即可访问 Rust 后端。
- README 已更新 Rust 启动、测试和构建命令。
