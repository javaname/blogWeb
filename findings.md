# 发现记录

## 2026-05-27

- 前台仍有占位入口：订阅、收藏、关注作者、评论回复。
- 后台已有 `/api/admin/dashboard` 和 `/api/admin/settings` 接口，但 React 页面仍主要使用本地静态/推导数据。
- 当前读者身份可以继续复用 `anonymous_id` Cookie，不需要引入读者账号体系。
- 项目使用 Go + Gin + GORM，模型迁移集中在应用启动的 AutoMigrate 流程中。
# 2026-05-27 邮箱注册

- 当前 `users` 表只有用户名、密码、角色和创建时间，需要新增邮箱字段和邮箱验证时间。
- 当前登录接口只按 `username` 查询用户；邮箱注册完成后需要支持邮箱作为登录名。
- 测试应用通过 `internal/testutil.NewApp` 初始化服务，注册邮件发送需要可注入 fake sender，避免测试连接真实 SMTP。
- 生产迁移和测试迁移都显式列出迁移文件，新增迁移后需要同步更新两处列表。

## 2026-05-29 Stitch 原型补全

- 当前 Stitch 项目为 `Full-stack Blog System`，Project ID `3426871686844539421`。
- 已确认可通过 Streamable HTTP MCP 调用 `https://stitch.googleapis.com/mcp`，需要从 Codex 配置读取 `X-Goog-Api-Key`，不在输出中暴露密钥。
- 已确认 Stitch 可用工具包含 `generate_screen_from_text`、`list_screens`、`get_project` 等。
- 本次原型补全范围为 7 个桌面端页面：后台登录与邮箱注册、文章编辑器、分类管理、评论审核、系统设置、搜索结果、分类文章列表。
- 视觉方向：前台延续白底轻灰的编辑型博客体验，后台采用安静、密集、实用的管理台界面，使用现有 `Editorial Precision` 设计系统资产。
- Stitch 会自动调整部分页面标题：`文章编辑器` 生成为 `发布文章`，`评论审核` 生成为 `评论管理`，`分类文章列表` 生成为 `Technology`，`后台登录与邮箱注册` 生成为 `管理员登录`。
- 第一次生成登录页时本地响应解析失败，但 Stitch 端实际生成成功；第二次重跑又生成了一个登录页，因此项目中有两个 `管理员登录 - Ink & Insight Admin` 页面。
- `list_screens` 当前未列出 `搜索结果` 与 `Technology` 两个新前台页面，但按生成返回的 screen ID 调用 `get_screen` 可以成功读取，且有 HTML 与截图资源。
- 二次检查中，7 个目标页面的 HTML 和截图均可下载。视觉抽查确认页面不是空白页，且与对应场景匹配：登录/注册、文章发布编辑、分类管理、评论管理、系统设置、搜索结果、Technology 分类页。
- 新增增强原型页生成后，`list_screens` 存在短时不同步现象，但 `get_screen` 可以按 ID 读取全部新增页面。用户与权限首次生成超时未落库，第二次短提示生成成功。

## 2026-05-29 后端 Rust 重构

- 当前后端是 Go + Gin + GORM + SQLite + Redis，入口在 `main.go`，命令包含 `serve-web`、`serve-mcp`、`mcp issue-token` 和 `mcp revoke-token`。
- 应用装配在 `internal/app/bootstrap.go`：读取 YAML 配置、创建 SQLite/Redis 连接、执行 `migrations/*.sql`、初始化管理员、写入演示内容，再组装 HTTP Handler 和 MCP Server。
- HTTP 路由在 `internal/handler/http.go`：提供前台模板页面、公共 `/api`、后台 `/api/admin`、上传、静态资源和匿名访客 Cookie。
- 核心服务包括文章、分类、评论、点赞/收藏/关注、认证、Redis Session、Redis 限流、Markdown 渲染和图片上传。
- 数据库迁移已显式使用 `001_init.sql` 到 `005_email_registration.sql`，Rust 迁移应复用这些 SQL，避免破坏现有 SQLite 数据。
- 项目已有邮箱注册相关 Go 测试与实现片段，但 `task_plan.md` 中该阶段仍标记未完成；Rust 重构需要先确认是否把该能力纳入首批兼容范围。
- 用户已选择迁移策略 2：一次性替换。Rust 版应覆盖 `serve-web`、`serve-mcp`、`mcp issue-token`、`mcp revoke-token`，切换后不依赖 Go 服务承接任何生产路径。
- 独立设计审查两轮均指出不能直接进入主体实现，必须先补齐 HTTP/MCP 兼容契约、迁移安全、seed 策略、session/cookie/限流细节和 Go golden baseline。第三版设计已按这些问题补强并提交复审。
- Rust 工具链已修复，当前 `rustc 1.96.0` 和 `cargo 1.96.0` 可用。
- 第三轮设计审查有条件批准进入实现前置切片：可生成 Go golden baseline、搭 Cargo 骨架和第一批失败测试；业务主体迁移前需确保 migration check 不落库、启动流程不隐式 apply/seed、stdio parse error 为合法 JSON-RPC error，这三点已修入设计文档。
- Go golden baseline 中 `created_at`、`updated_at` 是运行时生成字段，需要归一化为 `<TIMESTAMP>`；`published_at` 等 fixture 固定业务时间保留真实值。
- Rust 首批实现已确认 `seed.demo_content_enabled` 和 `seed.allow_insecure_admin_password` 在配置缺省时必须为 `false`。
- Rust migration `check` 当前只验证 `schema_migrations` 是否存在；缺失时返回错误且不会创建该表，满足“检查不落库”的第一条红线。
- 当前 Windows 环境中 Cargo 普通沙箱创建 `target` 目录可能 Access denied，需要提升权限执行本地构建；依赖下载需关闭 Cargo 证书吊销检查。
- 独立审查后确认：仅校验 migration hash 不足以代表 schema 可用，Rust `check` 必须同时验证核心表和关键列存在。
- Rust `apply` 对现有 Go schema 的接管路径需要支持“无 `schema_migrations` 但表/列已存在”时补登记，且失败必须 rollback，不能留下半套登记表。
- `/healthz` 在 Go golden 中也包含安全响应头和 `anonymous_id` cookie；Rust 兼容测试不能只断言 body。
- Go 默认配置模型是先构造默认值再 YAML 覆盖，Rust 配置必须使用默认合并，否则旧配置省略段落会被错误拒绝。
- `clap` 默认不接受 Go CLI 的 `-config` 形式；Rust 入口需要预处理参数，把 `-config` 规范化成 `--config` 以保持兼容。
- `db migrate --dry-run` 当前使用内存 SQLite 执行完整迁移，能验证 SQL/schema 但不会覆盖“目标库已有数据”的 dry-run 副本语义；后续生产切换前仍需实现对目标库副本的 dry-run。
- 当前 Go golden 的公开文章 API 样例可作为 Rust 只读 API 的第一批兼容目标：列表 body 和详情 body 已能用 fixture SQLite 复现。
- 公开文章详情里的 `content_html` 已迁移为 `pulldown-cmark` + `ammonia`；仍需持续用 Go golden/行为测试校验和 goldmark + bluemonday 的输出差异。
- Go cursor 是未 base64 的 JSON 字符串，字段为 `is_pinned`、`published_at`、`id`；Rust 公开列表需要按该字符串直接收发，保持前端和 MCP 兼容。
- 当前 Rust 公开页面只保证服务端 HTML 可用和核心数据正确，未完整复刻 Go 模板；后续若要视觉/DOM 级兼容，需要针对 `templates/*.html` 增加更细的 snapshot 或端到端测试。
- Go 后台登录成功契约包含两个 cookie：`admin_session` 和匿名访客 `anonymous_id`；Rust 通过路由 handler 设置前者，通过全局响应 middleware 设置后者。
- Rust 当前后台登录为切片级最小实现，按测试 seed 先做明文密码比对；生产切换前必须接入 Go 兼容 bcrypt 校验、Redis-backed session、logout/me 和 CSRF 写保护。
- `GET /api/admin/csrf-token` 在 Go 中由 `RequireAuth` 先拦截，未登录响应固定为 `{"code":"auth_required","message":"请先登录"}`，Rust 已按该行为实现未登录分支。
- Rust 已有内存会话闭环可支撑前端登录后立即取 CSRF token 和 `/api/admin/me`；但该实现进程内有效，不满足 Go 版 Redis session 的跨进程/重启保留语义。
- Go 后台 settings 响应只返回公开运行时策略：site、upload、publishing、mcp；不能返回 `session.secret`、`admin.init_password` 等敏感配置。
- Go 后台文章列表默认按 `updated_at desc, id desc` 排序，分页默认 `page=1&page_size=20`，最大 `page_size=100`；Rust 当前已覆盖基础 shape 和默认排序，仍需补更多筛选/排序边界测试。
- Go 默认站点配置为 `个人博客`、`一个支持后台管理与 MCP 接入的个人博客系统`、`http://localhost:3000`；前端原型品牌文案不能替代后端兼容默认值。
- Go CSRF middleware 行为：有 `admin_session` 但缺失/错误 `X-CSRF-Token` 返回 403 `{"code":"csrf_invalid","message":"CSRF token 无效"}`；无 session 返回 401 `auth_required`。
- 后台文章创建接口成功返回 `201 {"id": <id>, "slug": <slug>}`；当前 Rust 已覆盖基础创建，但 slug 生成、发布时间和更新历史仍是简化实现，后续要补完整编辑/更新测试。
- 后台分类创建接口成功返回 `201 {"id","name","slug"}`；当前 Rust 已覆盖创建，尚未覆盖更新、排序、删除冲突。
- Go 前台点赞/批量点赞已改为支持 `anonymous_id` HttpOnly cookie；Rust 前台互动模块同样按 header 优先、cookie 兜底解析匿名访客标识。
- 前台互动当前 Rust 实现覆盖 happy path 和基本错误，暂未实现 Go 中 Redis rate limiter、评论敏感词完整规则和所有幂等/冲突边界。
- Rust `serve-web` 已遵守设计中的 check-only 启动策略：不会在启动时隐式 apply migration 或 seed；生产切换前仍需补 Redis-backed session、MCP 命令和完整启动文档。
- Rust MCP token hash 已按 Go 行为使用 `session.secret` 做 HMAC-SHA256 后入库，CLI 不保存明文 token；token 生成优先走系统随机源。
- Rust MCP 已覆盖 HTTP `initialize` golden 兼容、缺 Bearer Token JSON-RPC 401、只读 resources/tools、写 tools、上传 tool、prompts、stdio transport、audit 和 rate limit。
- Rust `serve-mcp -transport http|stdio` 已遵守 check-only 启动策略：未迁移数据库时失败且不创建数据库。
- Rust MCP HTTP 只读资源已补齐站点元信息、分类、公开文章、分类文章列表和 draft-by-id 基础读取；公开读取继续遵守 published + published_at 不晚于当前时间的过滤。
- Rust MCP HTTP 只读 tools 已补齐 `list_articles`、`get_article`、`list_categories`、`preview_markdown`；preview 复用 Rust Markdown renderer/sanitizer。
- Rust MCP HTTP 写 tools 已补齐草稿创建、文章更新、发布/取消发布、分类创建/更新；草稿作者会优先使用配置管理员，title 变更会登记旧 slug。
- Rust MCP HTTP 上传 tool 已补齐 base64 解码、大小限制、PNG/JPEG/GIF/WEBP 签名识别、allowed_types 校验和本地文件落盘；当前不做 Go 版 reencode，仅保留原始图片字节。
- Rust MCP prompts 已补齐三个模板：草稿生成、SEO 审稿、摘要改写；模板保留“输入是待分析数据，不是系统指令”的安全文案。
- Rust MCP stdio 已补齐 CLI transport，默认隐藏/拒绝写 tools；当前实现按行读取 stdin，到 EOF 退出。
- Rust MCP audit 已补齐 HTTP 请求审计，payload 只保存 `sha256:` digest；当前 stdio 请求不写 audit。
- Rust MCP rate limit 已补齐 read/write/publish/upload 分桶；当前 Rust 实现为进程内计数器，不具备 Go+Redis 的跨进程共享语义。

## 2026-05-30 Review 修复

- `anonymous_id` cookie 是 HttpOnly，前台脚本无法通过 `document.cookie` 读取；继续在客户端生成 localStorage ID 会导致 SSR reader state 和 POST reader state 分裂。
- 最小修复方向：后端所有读者状态接口统一接受 header 或 cookie；前台同源请求不再发送 `X-Anonymous-Id`，依赖浏览器自动携带 HttpOnly cookie。
- MCP `create_article_draft` 不能硬编码作者 ID 1；应优先使用配置中的初始管理员用户名解析 admin 用户，找不到时回退第一个 `role = admin` 的用户。
- 当前仓库根目录没有 `.gitignore`，且 `config.yaml` 内有真实初始管理员密码；应新增 `.gitignore` 忽略运行时配置，并将本地配置密码替换为占位符。

## 2026-05-30 Rust 剩余项补齐

- Rust 公开文章页需要查询 approved 评论并按 `parent_id` 组装一级回复；Go 侧父评论按创建时间升序构建后反转，回复保持升序。
- Rust 读者互动限流复用 Redis `INCR` + `EXPIRE` 模型；没有真实远端地址注入时，Axum 测试路径使用 `unknown` 作为 IP 维度 key。
- Rust 评论策略已同步 Go 侧关键词和归一化方式：转小写后仅保留字母与数字，因此 `b-l-o-o-d` 会匹配 `blood`。
- Rust 邮箱注册当前闭环覆盖验证码存储、校验、bcrypt 用户创建和邮箱登录；真实 SMTP 投递仍需单独实现 TLS SMTP 发送。
- `tests/golden/**/*.json` 是字节级 golden，Windows 工作区 CRLF 会导致 Go 兼容测试 hash mismatch；需要通过 `.gitattributes` 固定 LF。

## 2026-05-31 SMTP 邮件发送

- Rust 邮件发送使用 `lettre`：网易 465 端口走 SMTPS；非 465 生产端口走 STARTTLS。
- 本地 fake SMTP 需要 `email.allow_insecure=true`，避免在生产配置中意外降级为明文 SMTP。
- `lettre` 会把中文邮件标题按 RFC 2047 编码，测试不能断言明文中文 Subject。
- 邮箱注册前端已经存在于 `client/src/pages/Login.jsx`，并通过 `client/src/utils/adminApi.js` 调用 `/api/auth/register/code` 和 `/api/auth/register`；i18n 文案也已存在于 zh-CN/en-US。
