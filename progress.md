# 进度记录

## 2026-05-27

- 用户确认选择 C：前台读者互动和后台管理闭环都做。
- 已创建任务计划、发现记录和进度记录。
- 已写入设计文档 `docs/superpowers/specs/2026-05-27-blog-remaining-features-design.md`。
- 进入后端 TDD 阶段，先补公共互动接口测试。
- 已新增后端互动接口测试并确认 RED：订阅、收藏/关注、评论回复。
- 已实现订阅、收藏、关注、评论回复数据结构、迁移、服务和公共接口。
- 已接入公共页面脚本：订阅、收藏、关注、评论回复均调用真实接口。
- 已接入后台 Dashboard/Settings：读取真实 admin API，设置页可保存站点基础配置。
- 已通过 `npm run check:i18n`、`npm run check:ui`、`npm run build` 和 `go test ./... -count=1 -timeout=120s`。
# 2026-05-27 邮箱注册

- 用户确认采用方案 A：邮箱验证码验证后创建账号。
- 已写入设计文档 `docs/superpowers/specs/2026-05-27-email-registration-design.md`。
- 下一步按 TDD 先补后端失败测试，再实现网易邮箱 SMTP 配置与注册接口。

## 2026-05-29 Stitch 原型补全

- 用户要求继续执行原型补全，基于已确认的 7 页范围直接生成 Stitch 页面。
- 已恢复本地计划文件，并追加本次 Stitch 原型补全阶段。
- 已通过 Stitch Streamable HTTP MCP 生成 7 个桌面端页面原型，并保存本地调用结果到 `.codex-run/stitch_prototype_generate.results.json`。
- 已通过 `get_screen` 按 ID 核验 7 个目标页面均存在，且都有 HTML 与截图资源；核验结果保存到 `.codex-run/stitch_get_generated_screens.results.json`。
- 生成页面 ID：管理员登录 `7cae317d534547e8ae887c1ae7aa0342`，发布文章 `64b92191575849c09a4859799f9221c8`，分类管理 `ea9ad058647e4d0e9bb69095b0b54c11`，评论管理 `29148e74563f4fa3baf3a3dbf91f92ae`，系统设置 `19a417f0f5b74fd0b6626481eb577430`，搜索结果 `30710adf00be4f28920fce3d7dd8ffc9`，Technology 分类页 `4f673048b57540458bd70214d42c5b56`。
- 注意：项目中另有一个首次失败调用遗留的重复登录页 `161657ce2d24467f96966be13a01dbf5`，当前 Stitch 工具列表未提供删除屏幕工具。
- 用户要求检查一遍后，已重新调用 `get_screen`、下载 7 个目标页的 HTML 与截图，并完成视觉抽查。7 个页面均可访问、资源非空、截图非空白，审计结果保存到 `.codex-run/stitch_audit_generated_screens.results.json` 和 `.codex-run/stitch-audit/`。
- 用户确认继续补增强原型页面后，已在 Stitch 项目 `Full-stack Blog System` 生成 8 个新增桌面端页面：关于我们、作者主页、标签文章列表、文章归档、404 页面、媒体库、用户与权限、数据分析。
- 已通过 Stitch `get_screen` 逐个核验 8 个新增页面均存在，且都有 HTML 与截图资源。新增页面 ID：关于我们 `a60e7ed6719246548e9aba707ba8cab3`，作者主页 `80e472182b1b4b88818b7c33d96b25e4`，标签文章列表 `2f6de06790ff4b4ab9fd68a642ed116c`，文章归档 `ad8a1e1b17f14839af57c9870fd0f943`，404 页面 `258db1d51f334ac48fbcdb6eee1f400a`，媒体库 `430bbf8ae9424101bf003040fdaae0b3`，用户与权限 `3fb4dd63176844f8ac242b38c1f854bd`，数据分析 `a112b7b18efb47f8a8e60fc5bd159eaf`。

## 2026-05-29 后端 Rust 重构

- 用户提出将当前 Go 后端重构为 Rust。
- 已激活 Serena 项目并读取说明，确认 CodeGraph 索引可用。
- 已恢复本地 `task_plan.md`、`findings.md`、`progress.md`，并发现旧的邮箱验证码注册阶段仍处于未完成状态。
- 已初步盘点 Go 后端架构：`main.go` 命令入口、`internal/app/bootstrap.go` 装配流程、`internal/handler/http.go` HTTP 路由、`internal/service/*` 核心服务、`migrations/*.sql` SQLite schema。
- 当前遵循 brainstorming 硬门禁：先完成设计与用户确认，再开始 Rust 实现。
- 用户选择方案 2：一次性替换。已更新计划状态，下一步输出 Rust 迁移设计草案并等待确认。
- 用户授权设计完成后无需过目，并授权独立子智能体审查新设计。
- 已新增设计文档 `docs/superpowers/specs/2026-05-29-rust-backend-rewrite-design.md`。
- 独立子智能体前两轮复审均未批准直接进入主体实现；已按意见补强第三版设计，覆盖字段契约、migration check/dry-run/apply、seed 显式开关、cookie/限流细节、MCP schema/token/audit 和 golden 归一化规则，并提交第三轮复审。
- Rust 工具链安装过程中出现 stable toolchain manifest 损坏，已卸载并重新安装修复；`rustc --version` 与 `cargo --version` 已可用。
- 第三轮复审有条件批准实现前置切片，不批准直接进入业务主体迁移。已修复剩余 3 个设计歧义：`check` 不落库、启动流程不隐式迁移/seed、stdio JSON 解码失败返回合法 JSON-RPC error。
- 已新增 Go golden baseline 生成测试 `internal/compat/golden_test.go`，生成并稳定验证 `tests/golden/http/*.json` 与 `tests/golden/mcp/*.json`。
- 已按 TDD 创建 Rust Cargo 工程骨架和首批测试：配置默认值、migration check 不落库、`GET /healthz` 兼容响应。
- 已观察并修复预期 RED：`seed` 缺省值先失败后改为安全默认 `false`；`/healthz` 先返回 `not-ready`，后改为 `{"status":"ok"}`。
- 已通过 `cargo test --offline`，以及 `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 注意：Rust 依赖首次下载很慢，需 `CARGO_HTTP_CHECK_REVOKE=false` 规避 Windows 吊销服务器离线问题；Go 旧 `.codex-run/go-build` 缓存出现过 Access denied，已用 `.codex-run/go-build-rust-slice` 验证通过。
- 独立审查指出首批实现存在 migration `check` 不验证真实 schema、`/healthz` 未对齐 golden header/cookie、配置不是 Go 默认合并模型、admin 默认密码策略未落地、golden token 归一化过宽等问题。
- 已按 TDD 修复审查高风险项：
  - migration `check` 增加核心表/列 smoke check。
  - migration `apply` 支持空库创建、已有 Go schema 补登记、hash mismatch 拒绝和失败回滚。
  - `/healthz` 测试读取 Go golden，Rust 响应补齐安全头、`Content-Type` charset 和 `anonymous_id` HttpOnly cookie。
  - Rust 配置改为 `#[serde(default)]` 默认合并模型，新增 `admin` 配置和不安全默认密码拒绝策略。
  - Go golden JSON 字符串不再按泛化 token 正则替换，仅保留明确 token 字段和 cookie 值归一化。
- 已再次通过 `cargo test --offline` 和 `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 已完成切片 A 后半的 CLI DB 命令入口：
  - `blogweb db check -config <path>` 只读检查，目标库不存在时失败且不落库。
  - `blogweb db migrate --dry-run -config <path>` 在内存库执行迁移，不创建目标库。
  - `blogweb db migrate --apply -config <path>` 创建 SQLite schema 并登记 migration hash，之后 `db check` 通过。
- 已新增 `tests/cli_db.rs` 覆盖以上 CLI 行为，并兼容 Go 风格单横线 `-config`。
- 最新验证通过：`cargo test --offline`、`go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 已开始切片 B：新增 `src/http_public.rs` 和 `tests/public_articles.rs`，覆盖 `GET /api/articles?limit=2` 与 `GET /api/articles/rust-migration-baseline` 的 Go golden body 兼容。
- 当前公开文章实现已迁移基础 SQLite 查询、分类/作者嵌套字段、点赞数、发布时间排序、详情状态字段和安全响应契约。
- 已补公开文章行为测试：隐藏草稿和未来发布时间、`category`/`keyword` 筛选、历史 slug 命中时 301 到当前 slug。
- 已补 cursor 分页契约：按 Go cursor JSON 结构 `is_pinned/published_at/id` 生成 `next_cursor`，并支持下一页查询。
- 已把公开文章详情的样例驱动 Markdown 渲染替换为 `pulldown-cmark` + `ammonia`，新增 `tests/renderer.rs` 覆盖 script 清理、GFM table、safe link 和 excerpt。
- 已新增公开页面/静态资源基础兼容：`/`、`/articles/:slug`、`/categories/:slug`、`/assets/*path`、`/uploads/*path`，并覆盖 HTML content-type、文章 HTML 不转义、分类过滤和路径穿越拒绝。
- 已知简化：页面 HTML 目前是 Rust 最小服务端输出，尚未复刻 Go 的完整 `templates/*.html`、相关文章、评论和前台视觉细节。
- 最新验证继续通过：`cargo test --offline`、`go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 已开始切片 C：新增 `tests/admin_auth.rs`，先确认 RED：`/api/admin/csrf-token` 和 `/api/admin/login` 均因缺路由返回 404。
- 已实现 Rust 后台认证最小兼容：新增 `src/admin_auth.rs`，挂载 `POST /api/admin/login` 与 `GET /api/admin/csrf-token`，登录成功返回 Go golden body 并设置 `admin_session` cookie，未登录 CSRF 返回 `auth_required`。
- 已验证 admin auth GREEN：`cargo test --offline --test admin_auth`。
- 已重新通过全量验证：`cargo test --offline`、`go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 已补后台会话闭环测试并确认 RED：登录后带 `admin_session` 请求 `/api/admin/csrf-token` 仍返回 401。
- 已扩展 Rust state 为共享内存会话表，登录时写入 session 和 CSRF token，新增 `GET /api/admin/me`，会话闭环测试 GREEN。
- 已再次通过 `cargo test --offline --test admin_auth`、`cargo test --offline`、`go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 已开始切片 D：新增 `tests/admin_read.rs`，先确认 RED：后台只读路由缺失导致 `/api/admin/dashboard` 未登录返回 404，登录后 dashboard/settings/articles/categories/comments 均无法解析 JSON。
- 已新增 `src/admin_read.rs`，实现后台只读 API：
  - `GET /api/admin/dashboard` 返回文章/评论/点赞统计、活动列表、30 天趋势。
  - `GET /api/admin/settings` 返回 site/upload/publishing/mcp 公开配置，不暴露 session secret 或初始密码。
  - `GET /api/admin/articles`、`GET /api/admin/categories`、`GET /api/admin/comments` 返回 Go 风格列表 shape。
- 已扩展 Rust 配置默认值，补入 Go 兼容 `site` 和 `mcp` 公开配置段。
- 已验证后台只读 GREEN：`cargo test --offline --test admin_read`。
- 已再次通过全量验证：`cargo test --offline`、`go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 已开始切片 E：新增 `tests/admin_write.rs`，先修复测试 raw string 语法后确认真实 RED：后台写接口缺失导致空 404 响应无法解析 JSON。
- 已新增 `src/admin_write.rs`，实现共享 CSRF 校验和三个最小写路径：
  - `POST /api/admin/categories` 创建分类。
  - `POST /api/admin/articles` 创建文章，复用 Rust renderer 生成 excerpt，拒绝外部 `http://` 封面。
  - `PUT /api/admin/comments/:id/status` 更新评论审核状态。
- 已验证写接口 GREEN：`cargo test --offline --test admin_write`。
- 已再次通过全量验证：`cargo test --offline`、`go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 已开始切片 F：新增 `tests/public_interactions.rs`，确认 RED：前台互动路由仍缺失，返回空 404。
- 已新增 `src/http_interactions.rs`，实现前台读者互动最小兼容：
  - 点赞与批量点赞状态，支持 `anonymous_id` cookie 和 `X-Anonymous-Id`。
  - 收藏文章、关注作者、邮件订阅、创建评论。
  - 响应 shape 对齐 Go 服务 `LikeResult`、`BookmarkResult`、`FollowResult`、`SubscribeResult` 和评论创建响应。
- 已验证互动接口 GREEN：`cargo test --offline --test public_interactions`。
- 已再次通过全量验证：`cargo test --offline`、`go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。
- 已开始切片 G：新增 CLI 测试 `serve_web_fails_when_database_is_not_migrated_without_creating_db`，确认 RED：Rust binary 不识别 `serve-web`。
- 已实现 Rust `serve-web` 命令：
  - 加载配置。
  - 目标数据库不存在时直接失败，不创建、不迁移。
  - 对已有库执行 `db::check_migrations`。
  - check 通过后用 axum 绑定 `0.0.0.0:<server.port>` 启动 HTTP server。
  - `normalized_args` 兼容 Go 默认命令行为：无子命令或首参数为 flag 时注入 `serve-web`。
- 已验证 CLI 启动前检查 GREEN：`cargo test --offline --test cli_db serve_web_fails_when_database_is_not_migrated_without_creating_db`。
- 已再次通过全量验证：`cargo test --offline`、`go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`。

## 2026-05-30 Review 修复与后台原型落地

- 用户要求 review 完毕后继续执行任务，并授权独立子智能体多线并行。
- 已启动两个 explorer：分别检查匿名访客 ID 一致性和 MCP 草稿作者硬编码问题。
- 已按 TDD 新增两个失败测试并确认 RED：
  - `TestLikeEndpointsUseAnonymousCookieWhenHeaderMissing` 当前因缺少 `X-Anonymous-Id` 返回 400。
  - `TestMCPHTTPCreateDraftUsesExistingAdminAuthor` 当前因硬编码 `AuthorID: 1` 触发外键失败。
- 已开始修复：后端匿名 ID 解析改为 header/cookie 统一入口，前台 `site.js` 移除本地生成匿名 ID 的逻辑。
- 已完成 review 修复：
  - 新增 `.gitignore` 忽略 `config.yaml`、`data/`、`.codex-run/`，并把本地 `config.yaml` 的 session secret 和 admin 初始密码替换为占位值。
  - `internal/handler/http.go` 新增统一匿名 ID helper，like/batchLikes 等接口支持 HttpOnly cookie。
  - `public/assets/site.js` 不再生成 localStorage 匿名 ID 或发送 `X-Anonymous-Id`，改为依赖同源 cookie。
  - `internal/mcp/tools_write.go` 新增 `defaultAuthorID`，MCP 草稿优先使用配置管理员，找不到则回退第一个 admin。
- 已验证 review 修复：
  - `go test ./internal/handler -run TestLikeEndpointsUseAnonymousCookieWhenHeaderMissing -count=1`
  - `go test ./internal/mcp -run TestMCPHTTPCreateDraftUsesExistingAdminAuthor -count=1`
  - `go test ./internal/handler ./internal/mcp -count=1`
  - `go test ./... -count=1 -timeout=120s`
- 已完成后台原型落地的主题能力：
  - 新增 `ThemeProvider` 和 `ThemeSwitcher`。
  - 登录页与后台顶栏均保留语言切换并新增主题切换。
  - `styles.css` 迁移为主题变量，并提供 `[data-theme='dark']` 深色变量。
  - `check-ui-completeness.mjs` 新增主题落地断言。
- 已验证前端：
  - `npm --prefix client run check:i18n`
  - `npm --prefix client run check:ui`
  - `npm --prefix client run build`

## 2026-05-30 Rust MCP 迁移

- 已继续切片 G，按 TDD 新增 `tests/mcp.rs`，先确认 RED：Rust crate 尚未导出 `mcp` 模块。
- 已新增 `src/mcp.rs` 并导出模块，实现 MCP 最小 HTTP/CLI 闭环：
  - `mcp issue-token`：校验已迁移数据库，生成 token，按 Go 兼容 HMAC-SHA256 hash 存入 `mcp_clients`，stdout 输出 `name/transport/token`。
  - `mcp revoke-token`：校验已迁移数据库，将指定 client 的 `is_enabled` 置为 `0`。
  - MCP HTTP `/mcp`：按 JSON-RPC 解码、协议版本/content-type/accept/origin/bearer token 校验，支持 `initialize` 和 `resources/list`，缺 token 与初始化响应对齐 Go golden。
  - `serve-mcp -transport http`：启动前只做 migration check，未迁移时失败且不创建数据库。
- 已扩展 Rust `McpConfig` 默认值，补入 `protocol_versions: ["2025-11-25"]` 并校验 `mcp.http_path` 和协议版本非空。
- 已把 token 生成改为优先使用系统随机源：Windows 使用 `BCryptGenRandom`，Unix 使用 `/dev/urandom`，失败时回退到带 session secret 的 HMAC 种子。
- 已验证：
  - `cargo test --offline --test mcp`
  - `cargo test --offline`
  - `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`
- 已继续按 TDD 补 MCP 只读资源与只读工具：
  - 新增失败测试覆盖 `resources/read` 的 `blog://site/meta`、`blog://categories`、`blog://articles/{slug}`、`blog://categories/{slug}/articles`，并确认未来发布时间文章不泄露。
  - 新增失败测试覆盖 `tools/list` 和只读 `tools/call`：`list_articles`、`get_article`、`list_categories`、`preview_markdown`。
  - 新增失败测试覆盖 invalid origin、invalid accept 和 scope 不足时的 Go 兼容鉴权失败行为。
- 已扩展 `src/mcp.rs`：
  - 实现 MCP resources/read 公开只读资源。
  - 实现只读 tools 及 Markdown preview sanitizer。
  - 实现 scope 不足时 `WWW-Authenticate: Bearer error="insufficient_scope"`，401 时返回 Bearer challenge。
- 最新验证继续通过：
  - `cargo test --offline`
  - `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`
- 已继续按 TDD 补 MCP 写工具（暂不含上传）：
  - 新增失败测试覆盖 `create_article_draft` 使用配置管理员作者而非硬编码 ID、`update_article` 变更 title 后写入 slug history、`publish_article` 后可通过 MCP 公开读取。
  - 新增失败测试覆盖 `create_article_draft` 拒绝外部 `http://` 封面和路径穿越封面。
  - 新增失败测试覆盖 `create_category` 与 `update_category`。
- 已实现对应 MCP 写工具：
  - 文章草稿创建、文章更新、发布/取消发布。
  - 分类创建、分类更新。
  - slugify、唯一 slug、旧 slug history、默认管理员作者选择、title/category/markdown/cover_image 校验。
- 最新验证继续通过：
  - `cargo test --offline --test mcp`
  - `cargo test --offline`
  - `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`
- 已更新运行文档：
  - `README.md` 改为 Rust 启动、迁移、MCP token 和验证命令。
  - `docs/mcp-client.md` 补充 Rust `serve-mcp`、token 管理、stdio 默认只读、进程内限流和上传不 reencode 的实现说明。
- 已继续按 TDD 补 MCP audit 与 rate limit：
  - 新增失败测试覆盖 HTTP 成功请求和缺 Bearer Token 拒绝请求写入 `mcp_audit_logs`，且 payload 只保存 `sha256:` digest，不包含原始 payload。
  - 新增失败测试覆盖 read 与 upload 分桶限流，超过限制返回 429 `rate_limited`。
- 已实现 Rust MCP HTTP audit：记录 client_id、transport、action_type、target、scope、status、request_id、error_code、payload_digest。
- 已实现 Rust MCP HTTP 进程内 rate limit：read/write/publish/upload 分桶，配置来自 `mcp.rate_limit`，默认值对齐 Go。
- 最新验证继续通过：
  - `cargo test --offline --test mcp`
  - `cargo test --offline`
  - `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`
- 已继续按 TDD 补 `serve-mcp -transport stdio`：
  - 新增失败测试通过 CLI stdin 喂入 `tools/list`、`preview_markdown` 和写 tool 请求，要求 EOF 后进程正常退出。
  - 默认 `mcp.stdio_write_enabled=false` 时，`tools/list` 不暴露写 tools，写 tool 返回 403 `forbidden_scope`。
- 已实现 Rust MCP stdio transport：读取 stdin JSON-RPC 请求，复用 MCP dispatch，逐行输出 JSON-RPC response；写能力由 `mcp.stdio_write_enabled` 控制。
- 最新验证继续通过：
  - `cargo test --offline --test mcp`
  - `cargo test --offline`
  - `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`
- 已继续按 TDD 补 MCP prompts：
  - 新增失败测试覆盖 `prompts/list` 返回 `draft_article_from_outline`、`seo_review_article`、`rewrite_article_summary`。
  - 新增失败测试覆盖 `prompts/get` 返回 `{name, content, input}` 并校验 title/content。
- 已实现 Rust MCP prompts 三个模板，文案保持 Go 侧语义：输入是待分析数据，不可作为执行指令，落库必须由客户端显式调用 tool。
- 最新验证继续通过：
  - `cargo test --offline --test mcp`
  - `cargo test --offline`
  - `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`
- 已继续按 TDD 补 MCP `upload_image`：
  - 新增失败测试覆盖有效 PNG base64 上传落盘并返回 `/uploads/...` URL。
  - 新增失败测试覆盖伪装非图片返回 415 `unsupported_media_type`。
  - 新增失败测试覆盖超过 `upload.max_size` 返回 413 `payload_too_large`。
- 已实现 Rust MCP 上传 tool：base64 解码、PNG/JPEG/GIF/WEBP 签名识别、allowed_types 校验、按 UTC 年月写入上传目录并返回 url/filename/mime_type/size。
- 最新验证继续通过：
  - `cargo test --offline --test mcp`
  - `cargo test --offline`
  - `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`

## 2026-05-30 Rust 后台认证补齐

- 用户要求补齐当前 Rust 后台认证中的最小内存会话和明文测试密码路径，完成后暂停复盘。
- 已按 TDD 新增认证测试支持：
  - `tests/support.rs` 提供 fake Redis RESP 服务和 bcrypt 管理员密码 fixture。
  - `tests/admin_auth.rs` 覆盖 bcrypt 登录、Redis `session:*`/`csrf:*` 写入、logout 删除 Redis key 并清 cookie。
  - `tests/admin_read.rs`、`tests/admin_write.rs` 改为通过 fake Redis session 验证后台只读和 CSRF 写接口。
- 已新增 `src/session.rs`，实现 Rust Redis session store：随机 24 字节 URL-safe token、session/csrf 双 key 写入、TTL、last_seen 刷新、绝对过期和 idle timeout 检查、destroy 删除双 key。
- 已改造 `src/admin_auth.rs`：登录使用 `bcrypt::verify`，session 写入 Redis，新增 `POST /api/admin/logout`，`/csrf-token` 和 `/me` 从 Redis 读取会话。
- 已改造后台只读/写接口为 async session 校验，写接口继续要求 `X-CSRF-Token`。
- 已新增 Rust 依赖：`bcrypt`、`base64`、`rand`，并启用 `tokio` 的 `net`/`io-util` features。
- 已验证通过：
  - `cargo test --offline --test admin_auth --test admin_read --test admin_write`
  - `cargo test --offline`
- 按用户要求，认证部分完成后暂停；未继续推进后台写接口补全、模板级页面复刻、邮箱注册。

## 2026-05-30 Rust 剩余 16 项并行补齐启动

- 用户要求启动独立子智能体并行推进复盘中未做的 16 项。
- 当前界面无独立子智能体调度工具，实际执行方式调整为：并行梳理上下文，按 TDD 将互相影响的文件编辑串行落地。
- 已在 `task_plan.md` 新增“Rust 剩余 16 项并行补齐”阶段，拆为后台 API、邮箱注册、公开模板、互动限流、验证提交 5 个切片。

## 2026-05-30 Rust 剩余 16 项补齐进展

- 已完成后台 API 补齐并覆盖测试：
  - `GET/PUT/DELETE /api/admin/articles/:id`
  - `PUT/DELETE /api/admin/categories/:id`
  - `PUT /api/admin/categories/sort`
  - `DELETE /api/admin/comments/:id`
  - `PUT /api/admin/settings`
  - `POST /api/admin/upload`
- 已完成邮箱验证码注册和邮箱登录闭环测试路径：`/api/auth/register/code`、`/api/auth/register`、注册用户 bcrypt 密码、邮箱作为登录名、Redis session 登录。当前 Rust 路径支持 fake/test sender；真实 SMTP 投递仍未实现。
- 已补公开文章页细节：文章页展示 approved 评论、一级回复和同分类相关文章；pending 评论不渲染。
- 已补读者互动限流与评论策略：
  - Rust 配置新增 Go 同名 `rate_limit` 默认值。
  - Redis 客户端新增 `INCR/EXPIRE`，fake Redis 同步支持测试。
  - 点赞、收藏、关注、订阅、评论和批量点赞进入 Redis 限流。
  - 评论敏感词策略同步 Go 侧政治/暴力/血腥关键词与归一化规则。
- 验证通过：
  - `cargo test --offline`
  - `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`
  - `go test ./... -count=1 -timeout=120s`
- 遇到并处理的问题：
  - `session-catchup.py` 本机路径不存在，已继续按现有计划文件恢复。
  - 一次 PowerShell regex 文件筛选写法错误，未影响代码。
  - Go golden JSON 工作区被 CRLF 化导致字节级 hash mismatch；已重写为 LF，并新增 `.gitattributes` 固定 `tests/golden/**/*.json text eol=lf`。
