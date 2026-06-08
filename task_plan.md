# 博客剩余功能补全计划

## 目标
把当前前台和后台页面上已经露出的缺失功能补成最小可用闭环：前台读者互动可真实请求并持久化，后台仪表盘和设置页使用真实接口。

## 阶段

1. [complete] 记录设计与实施计划
2. [complete] 补后端互动模型、接口和测试
3. [complete] 接入公共页面交互脚本
4. [complete] 接入后台仪表盘和设置页真实接口
5. [complete] 运行完整验证并修复失败

## 范围

- 邮件订阅：保存邮箱和匿名访客标识，不发送邮件。
- 收藏文章：按匿名访客和文章记录收藏状态，支持切换。
- 关注作者：按匿名访客和作者记录关注状态，支持切换。
- 评论回复：评论支持 `parent_id`，文章页按简单嵌套展示回复。
- 后台数据：仪表盘读取真实统计接口，设置页读取并保存站点基础配置。

## 非目标

- 不新增注册/登录给读者。
- 不接入真实邮件投递服务。
- 不做完整通知中心、作者主页或会员体系。

# 新增阶段：Stitch 原型补全

目标：在 Stitch 项目 `Full-stack Blog System` 中，根据当前项目已有前台模板和后台 React 页面，补全缺失的高保真桌面端页面原型。

Stitch 项目：
- Project ID: `3426871686844539421`
- Design system asset: `assets/13bf83c71f064e30b7ce946e303ff2d9`

待生成页面：
1. [complete] 后台登录与邮箱注册 - Ink & Insight Admin
2. [complete] 文章编辑器 - Ink & Insight Admin
3. [complete] 分类管理 - Ink & Insight Admin
4. [complete] 评论审核 - Ink & Insight Admin
5. [complete] 系统设置 - Ink & Insight Admin
6. [complete] 搜索结果 - Ink & Insight
7. [complete] 分类文章列表 - Ink & Insight

验证：
- [complete] 调用 Stitch `get_screen` 确认 7 个生成页面均有 HTML 与截图资源。
# 新增阶段：邮箱验证码注册

目标：新增邮箱注册能力，使用网易邮箱 SMTP 发送验证码，验证码校验通过后创建普通用户账号。

1. [complete] 写入邮箱注册设计文档与计划记录
2. [complete] 按 TDD 增加后端注册与验证码失败测试
3. [complete] 实现后端配置、迁移、邮件发送、注册接口和邮箱登录
4. [complete] 实现前端登录页注册表单和国际化文案
5. [complete] 运行 Go 与前端完整验证

# 新增阶段：后端 Rust 重构

目标：将当前 Go 后端逐步重构为 Rust 后端，在保持前端、SQLite 数据、配置文件、模板和公开 HTTP/MCP 行为兼容的前提下完成迁移。

当前状态：
- [complete] 盘点 Go 后端入口、路由、服务、数据表、配置和测试边界
- [complete] 确认迁移策略：一次性替换
- [complete] 写入并审查 Rust 迁移设计文档
- [complete] 生成 Go golden baseline，并按 TDD 创建 Rust 后端骨架和首批兼容性测试
- [complete] 切片 A 前半：配置默认合并、数据库迁移 check/apply、应用装配、`/healthz`
- [complete] 切片 A 后半：CLI `db check/migrate`、SQLite pool 装配、启动时 check-only 基础流程
- [complete] 切片 B：公开只读页面/API、模板渲染、文章列表、文章详情、分类页、静态资源
  - [complete] `GET /api/articles` 与 `GET /api/articles/:slug` 对齐当前 Go golden body
  - [complete] 分类筛选、keyword、隐藏草稿/未来发布、历史 slug 301
  - [complete] cursor 分页契约
  - [complete] 真实 Markdown renderer / sanitizer
  - [complete] 公开首页、文章页、分类页、assets/uploads 静态资源基础兼容
  - [complete] 文章页 approved 评论、一级回复和同分类相关文章基础展示
  - [complete] Go 模板级公开页面 DOM/交互钩子复刻（Rust renderer 实现）
- [complete] 切片 C：后台认证与会话兼容
  - [complete] `POST /api/admin/login` 对齐 Go golden body 与 `admin_session` cookie 契约
  - [complete] `GET /api/admin/csrf-token` 未登录对齐 Go golden 401 body
  - [complete] 登录后会话可读取 `GET /api/admin/csrf-token` 与 `GET /api/admin/me`
  - [complete] Redis 会话存储、bcrypt 密码校验、logout、CSRF 写接口保护
- [complete] 切片 D：后台管理只读 API
  - [complete] `GET /api/admin/dashboard` 真实统计、活动和 30 天趋势
  - [complete] `GET /api/admin/settings` 返回公开运行时策略且不泄露密钥
  - [complete] `GET /api/admin/articles`、`/categories`、`/comments` 列表基础兼容
  - [complete] `GET /api/admin/articles/:id` 编辑详情
  - [complete] 后台文章列表 status/category/keyword 筛选、like_count/非法排序、分页边界行为覆盖
- [complete] 切片 E：后台写接口与 CSRF
  - [complete] 后台写接口缺失/错误 CSRF token 返回 Go 兼容 403 `csrf_invalid`
  - [complete] `POST /api/admin/categories` 基础创建
  - [complete] `POST /api/admin/articles` 基础创建和外部 http 封面拒绝
  - [complete] `PUT /api/admin/comments/:id/status` 基础审核状态更新
  - [complete] 文章编辑详情、文章更新/删除、分类更新/排序/删除、评论删除、设置更新、上传接口
- [complete] 切片 F：前台读者互动 API
  - [complete] `POST /api/articles/:slug/like` 与 `POST /api/likes/batch` 支持 `anonymous_id` cookie
  - [complete] `POST /api/articles/:slug/bookmark`
  - [complete] `POST /api/authors/:id/follow`
  - [complete] `POST /api/newsletter/subscribe`
  - [complete] `POST /api/articles/:slug/comments`
  - [complete] 读者互动速率限制、敏感词完整策略、前台页面评论/回复/相关文章基础展示
- [complete] 切片 G：Rust 启动命令
  - [complete] `blogweb serve-web -config <path>` 启动前只做 migration check，未迁移时失败且不创建数据库
  - [complete] 支持 Go 风格默认命令：无子命令或首参数为 flag 时按 `serve-web` 解析
  - [complete] `blogweb serve-mcp -transport http -config <path>` 启动前只做 migration check，未迁移时失败且不创建数据库
  - [complete] `blogweb mcp issue-token` 使用 Go 兼容 HMAC token hash 入库，输出 `name/transport/token`
  - [complete] `blogweb mcp revoke-token` 将 MCP client 标记为禁用
  - [complete] MCP HTTP `initialize`、`resources/list`、`resources/read` 公开只读资源和 `tools/list`
  - [complete] MCP HTTP 只读 `tools/call`：`list_articles`、`get_article`、`list_categories`、`preview_markdown`
  - [complete] MCP HTTP 协议版本、Content-Type、Accept、Origin、Bearer token 和 scope 失败响应基础兼容
  - [complete] MCP HTTP 写 `tools/call`：`create_article_draft`、`update_article`、`publish_article`、`unpublish_article`、`create_category`、`update_category`
  - [complete] MCP HTTP 上传 `tools/call`：`upload_image` base64 解码、大小限制、图片类型识别和文件落盘
  - [complete] MCP HTTP prompts：`prompts/list`、`prompts/get` 三个模板和参数校验
  - [complete] `serve-mcp -transport stdio`：从 stdin 读取 JSON-RPC 到 EOF，默认隐藏/拒绝写能力
  - [complete] MCP HTTP audit：成功、失败、拒绝请求写入 `mcp_audit_logs`，payload 仅保存 digest
  - [complete] MCP HTTP rate limit：Redis 共享 read/write/publish/upload 分桶限流，Redis 不可用时本进程 fallback
  - [complete] 运行文档：README 与 MCP 客户端接入说明补充 Rust CLI 命令和行为差异
- [complete] 一次性迁移配置、数据库、认证会话、文章/分类/评论/互动、上传和 MCP 能力
- [complete] 更新启动文档与验证命令

# 新增阶段：Review 修复与后台原型落地

目标：先修复 review 指出的安全与正确性问题，再继续执行后台管理端 Stitch 原型落地。

当前状态：
- [complete] 修复 review 问题：配置密钥暴露、匿名访客 ID 不一致、MCP 草稿作者硬编码
- [complete] 运行后端定向测试和全量验证
- [complete] 按已批准规格实现后台管理端 Stitch 原型落地
- [complete] 运行前端 i18n、UI 和构建验证

# 新增阶段：Rust 剩余 16 项并行补齐

目标：基于复盘清单补齐 Rust 后端剩余生产路径能力，保持已有 Go/Rust 契约兼容。

执行方式：
- [complete] 切片 1：后台文章/分类/评论/设置/上传 API 补齐
  - [complete] `GET /api/admin/articles/:id`
  - [complete] `PUT /api/admin/articles/:id`
  - [complete] `DELETE /api/admin/articles/:id`
  - [complete] `PUT /api/admin/categories/:id`
  - [complete] `DELETE /api/admin/categories/:id`
  - [complete] `PUT /api/admin/categories/sort`
  - [complete] `DELETE /api/admin/comments/:id`
  - [complete] `PUT /api/admin/settings`
  - [complete] `POST /api/admin/upload`
- [complete] 切片 2：邮箱验证码注册与邮箱登录闭环
  - [complete] `POST /api/auth/register/code`
  - [complete] `POST /api/auth/register`
  - [complete] Rust email config / fake sender 测试路径
  - [complete] 注册用户 bcrypt 密码和邮箱登录
  - [complete] 真实 SMTP 投递实现
- [complete] 切片 3：公开页面模板细节补齐
  - [complete] 首页/分类页/文章页基础结构向 Go 模板靠齐
  - [complete] 文章页评论展示、回复展示和相关文章
- [complete] 切片 4：读者互动限流与策略
  - [complete] 点赞/收藏/关注/订阅/评论限流
  - [complete] 评论敏感词完整策略
- [complete] 切片 5：验证、文档记录、阶段提交与推送
  - [complete] `cargo test --offline`
  - [complete] `go test ./internal/compat -run TestGenerateGoldenBaseline -count=1`
  - [complete] `go test ./... -count=1 -timeout=120s`
  - [complete] 阶段提交与推送：`48f1e46`

# 新增阶段：远端 Stitch 快照与前端一致性审计

目标：重新拉取当前 Stitch 远端项目数据保存到本地，并判断当前项目前端功能是否与远端原型一致。

当前状态：
- [complete] 通过重新配置后的 Stitch MCP HTTP 配置拉取远端项目和 screen 列表。
- [complete] 保存 14 个远端 screen 的详情、HTML 和截图到 `stitch_current_snapshot/`。
- [complete] 抽取当前 React 后台路由和后端 SSR 前台路由。
- [complete] 完成功能一致性判断。

结论：
- 后台管理核心原型功能基本一致：登录/邮箱注册、控制台、文章管理、发布/编辑文章、分类管理、评论管理、系统设置均有对应 React 路由和真实 API。
- 前台核心阅读功能部分一致：博客首页、文章详情、分类文章页、搜索结果、订阅、点赞、收藏、关注作者、评论/回复通过服务端模板和 `public/assets/site.js` 支撑。
- 远端原型中仍有当前前端未完整落地的独立页面或入口：`关于我们`、`作者主页`、独立 `分类浏览` 页面，以及原型历史记录中提到但当前远端列表未出现的标签文章列表、归档、404、媒体库、用户与权限、数据分析。

# 新增阶段：基于 Stitch 快照补齐公开前端页面

目标：把当前远端 Stitch 快照中已存在但真实前端缺失的公开页面补成可访问 SSR 路由，并让公共导航进入这些页面。

当前状态：
- [complete] 按 TDD 新增公开页面断言，确认 `/categories`、`/about`、`/authors/1` 缺失时失败。
- [complete] 实现 `/categories` 分类浏览页，展示分类总数、文章总数、分类卡片和分类文章入口。
- [complete] 实现 `/about` 关于页，展示站点定位、编辑原则、订阅表单和分类入口。
- [complete] 实现 `/authors/:id` 作者主页，展示作者信息、关注按钮、文章数量和作者文章列表。
- [complete] 更新前台导航和页脚，把分类/关于从页面锚点改为真实路由。
- [complete] 验证通过：`cargo fmt --check`、`cargo test --offline`、`go test ./... -count=1 -timeout=120s`。

# 新增阶段：远端缺失页面补全与本地 Web 原型落地

目标：通过 Stitch MCP 补全远端缺失页面，重新同步远端快照到本地，并将缺失页面落地到本地公开 SSR 与后台 React 原型。

当前状态：
- [complete] Stitch 远端新增并核验 6 个页面：标签文章列表、文章归档、404、媒体库、用户与权限、数据分析。
- [complete] 本地快照已重新同步到 `stitch_current_snapshot/`，当前 screen 总数为 20，并包含新增页面的 HTML、截图和 raw JSON。
- [complete] 公开 SSR 已补齐 `/search`、`/tags/:slug`、`/archive` 和品牌 404 fallback。
- [complete] 后台 React 已补齐 `/media`、`/users`、`/analytics` 原型页、侧栏入口、英文文案和响应式样式。
- [complete] 验证通过：`cargo fmt --check`、`cargo test --offline --test public_pages_static`、`cargo test --offline`、`go test ./... -count=1 -timeout=120s`、`npm --prefix client run check:i18n`、`npm --prefix client run check:ui`、`npm --prefix client run build`。

# 新增阶段：退役 Go 后端实现

目标：对照 Go 与 Rust 后端实现，删除已由 Rust 完整重写的 Go 版本，保留静态 golden 作为兼容契约。

当前状态：
- [complete] 已确认 Rust 覆盖 Web、MCP、数据库迁移、认证会话、公开/后台 API、上传、读者互动和邮件注册等生产路径。
- [complete] 已确认 Rust 测试仅读取 `tests/golden/**/*.json` 静态 fixture，不再依赖运行 Go 代码。
- [complete] 已删除 Go 源码、Go 测试、`go.mod` 和 `go.sum`。
- [complete] 已同步 README、CLAUDE、前端 UI 检查脚本和安全测试说明，当前验证矩阵切换为 Rust/前端。

# 新增阶段：根据前端页面完善后端接口

目标：对照当前已落地的前端页面，把仍停留在静态原型或原型级数据的页面补成真实 Rust 后端接口闭环，优先服务后台管理端 `/media`、`/users`、`/analytics` 三个页面。

排产原则：
- 先补接口契约和后端 TDD，再改前端页面接入真实数据。
- 复用现有认证、CSRF、分页、错误响应和上传策略，不新增读者登录体系。
- 对数据库结构谨慎扩展；能从现有表聚合的数据优先聚合，确需新资源表时单独迁移。
- 每个切片完成后运行对应 Rust 测试和前端检查，并阶段提交推送。

当前状态：
- [complete] 盘点前端页面和后端路由缺口
- [complete] 排产接口范围与实施切片
- [pending] 切片 1：后台媒体库接口
  - [pending] `GET /api/admin/media`：返回上传资源列表、类型/大小/使用状态、分页和筛选
  - [pending] `GET /api/admin/media/stats` 或列表内 `stats`：返回文件数、存储占用、被文章使用数量、待补 alt 数量
  - [pending] `POST /api/admin/media`：复用现有上传策略，返回可用于文章封面的资源记录
  - [pending] `PUT /api/admin/media/:id`：更新 alt/title/usage 元数据
  - [pending] `DELETE /api/admin/media/:id`：未被文章引用时删除记录和文件，引用中资源拒绝删除
  - [pending] 前端 `/media` 从静态数组切换为真实 API，并保留空状态/加载/错误态
- [pending] 切片 2：后台用户与权限接口
  - [pending] `GET /api/admin/users`：返回管理员/编辑/普通用户列表、文章数、邮箱、角色和状态
  - [pending] `GET /api/admin/users/stats` 或列表内 `stats`：返回总成员、管理员、编辑、待邀请/待验证数量
  - [pending] `POST /api/admin/users/invitations`：预留邀请成员闭环，先支持创建 pending 用户或邀请记录
  - [pending] `PUT /api/admin/users/:id/role`：调整角色，禁止降级最后一个管理员
  - [pending] `PUT /api/admin/users/:id/status`：启用/禁用用户，禁止禁用当前会话用户和最后一个管理员
  - [pending] 前端 `/users` 从静态数组切换为真实 API，权限说明保留配置化静态文案
- [pending] 切片 3：后台数据分析接口
  - [pending] `GET /api/admin/analytics`：返回指标卡、30 天趋势、来源分布和热门内容
  - [pending] 基于现有文章、评论、点赞、收藏、关注、订阅等表聚合可用指标
  - [pending] 视图/来源/阅读时长等当前无事件表的数据先返回明确的估算或空数据来源标识
  - [pending] 前端 `/analytics` 从静态数组切换为真实 API，并处理无事件数据时的展示
- [pending] 切片 4：公开标签能力从原型筛选升级
  - [pending] 评估是否新增 tags/article_tags 表；若暂不新增，则明确 `/tags/:slug` 的派生规则和后台不可编辑边界
  - [pending] 如新增标签表，同步补后台文章编辑标签字段、公开标签列表和 MCP 只读资源
- [pending] 切片 5：验证、文档、提交和推送
  - [pending] `cargo fmt --check`
  - [pending] `cargo test --offline`
  - [pending] `npm --prefix client run check:i18n`
  - [pending] `npm --prefix client run check:ui`
  - [pending] `npm --prefix client run build`
- [pending] 阶段提交并推送到远程

# 新增阶段：notice.html 安全基线整改与接口收口

目标：对照 `notice.html` 的 P0/P1/P2 安全基线，先处理上线前必须完成的安全缺口，再补齐仍停留在原型数据的后台页面接口。

当前校准：
- 后台用户与权限基础接口已存在：`/api/admin/users`、`/api/admin/users/:id/role`、`/api/admin/users/:id`，React `/users` 已接入真实 API。
- 后台媒体库和数据分析仍未接入真实后端：`/media`、`/analytics` 仍使用静态数组，缺 `/api/admin/media` 和 `/api/admin/analytics`。
- 旧计划中的“用户基础接口全缺失”已滞后；后续只保留用户权限边界增强。

## P0 必须优先完成

1. [pending] 后台 RBAC 与资源归属收口
   - [pending] 为后台读/写接口建立统一权限 helper，区分 `admin`、`editor`、`writer`、`user`。
   - [pending] 文章创建/更新/删除按角色和作者归属校验；作者只能管理自己的文章，设置、分类、用户、上传策略等高危能力仅允许 admin 或明确授权角色。
   - [pending] 评论审核/删除、分类管理、设置更新、上传接口补服务端权限测试。
   - [pending] 用户角色调整增加“不能降级或删除最后一个管理员”测试与实现。
2. [pending] 登录与注册限流落地
   - [pending] 将 `RateLimitConfig` 的 `login_ip_*`、`login_user_*`、`registration_ip_*`、`registration_email_*` 接入 `login`、`request_registration_code` 和 `register_with_email`。
   - [pending] 按账号/IP/邮箱维度补 Redis 限流测试，错误响应统一为可理解业务错误。
3. [pending] Cookie 与生产响应头加固
   - [pending] `admin_session` 与 `anonymous_id` 增加 `SameSite` 策略，生产 HTTPS 模式增加 `Secure`。
   - [pending] 增加 HSTS、`Permissions-Policy`，并为敏感后台/API 响应补正确 `Cache-Control`。
   - [pending] 收紧 CSP，消除对 `unsafe-eval` 和 Tailwind CDN 的生产依赖；如需保留 inline style/script，先改为 nonce/hash 策略。
4. [pending] 上传安全补强
   - [pending] 让 `upload.reencode` 真正执行图片重解码/重编码，剥离 EXIF 元数据。
   - [pending] 增加图片宽高、像素总量和处理超时限制。
   - [pending] 建立媒体资源元数据表，记录 MIME、大小、alt/title、引用关系和上传者。
5. [pending] 关键操作审计与备份自动化
   - [pending] 后台登录、登出、发布、修改、删除、权限变更、设置更新、上传写入统一审计日志，敏感字段脱敏。
   - [pending] 在 `docs/backup-restore.md` 之外增加可执行备份/恢复演练脚本或任务说明，覆盖 SQLite、Redis、上传目录和配置。

## P1 建议完成

1. [pending] 后台媒体库真实 API 与 React 接入
   - [pending] `GET /api/admin/media`、`POST /api/admin/media`、`PUT /api/admin/media/:id`、`DELETE /api/admin/media/:id`。
   - [pending] 媒体统计返回文件数、存储占用、引用数、待补 alt 数量；前端 `/media` 移除静态数组。
2. [pending] 后台数据分析真实 API 与 React 接入
   - [pending] `GET /api/admin/analytics` 聚合文章、评论、点赞、收藏、关注、订阅等现有数据。
   - [pending] 对当前没有事件表支撑的访问量、来源、阅读时长返回明确的数据来源标识，避免伪造真实指标。
3. [pending] 内容安全与反垃圾增强
   - [pending] Markdown 外链自动加 `rel="nofollow ugc"`。
   - [pending] 评论链接、多链接内容、新用户评论进入审核/限流增强策略。
4. [pending] 幂等与重复提交保护
   - [pending] 后台关键写接口支持 `X-Request-Id` 或一次性提交 token。
   - [pending] 明确重复请求的响应码和响应体，补数据库唯一约束冲突的业务化错误。
5. [pending] 依赖与供应链扫描
   - [pending] 在 CI 中加入 `cargo audit` 或同类 Rust 依赖扫描。
   - [pending] 在前端加入 `npm audit`/Dependabot/Snyk 或同类扫描流程。

## P2 按需启用

1. [pending] 高级反爬策略：蜜罐链接、行为评分、设备指纹或风险挑战。
2. [pending] 搜索引擎和隐私页面索引策略：`robots.txt`、`X-Robots-Tag`、后台/草稿/私密资源禁止索引。
3. [pending] 发布包完整性：构建产物校验摘要、SBOM 和发布回滚记录。

验证矩阵：
- [pending] `cargo fmt --check`
- [pending] `cargo test --offline`
- [pending] 新增定向安全测试：RBAC、登录限流、Cookie/响应头、上传重编码与尺寸限制、审计日志
- [pending] `npm --prefix client run check:i18n`
- [pending] `npm --prefix client run check:ui`
- [pending] `npm --prefix client run build`
- [pending] 阶段提交并推送到远程

# 新增阶段：本地 PostgreSQL 迁移与数据同步

目标：将当前 Rust 后端从 SQLite-only 切换为本地 PostgreSQL-only，默认连接本机 `localhost:5432/blogweb`，并把当前项目 SQLite 数据库 `data/blog.db` 同步到 PostgreSQL 的 `blogweb` 库。

决策：
- 采用 PostgreSQL-only，不保留 SQLite 运行时兼容。
- 配置从 `database.path` 调整为 PostgreSQL 连接串，默认库为 `blogweb`。
- SQLite 只作为一次性数据同步源读取。
- 数据同步应先建表/迁移，再按外键顺序导入数据，并修复 PostgreSQL 序列。

当前状态：
- [complete] 用户确认采用推荐方案：PostgreSQL-only。
- [complete] 用户确认本地连接使用 `localhost:5432/blogweb`。
- [complete] 用户要求当前项目数据库数据同步到 `blogweb`。
- [complete] 追加任务计划、发现和进度记录。
- [complete] 切片 1：配置与数据库连接测试
  - [complete] 配置读取 `database.url`。
  - [complete] 默认连接串指向本地 `blogweb`。
  - [complete] `db check/migrate` 使用 PostgreSQL 连接，不检查 SQLite 文件路径。
- [complete] 切片 2：PostgreSQL 迁移 SQL
  - [complete] 将建表 SQL 改为 PostgreSQL 方言。
  - [complete] `schema_migrations`、核心表、关键列检查使用 PostgreSQL 系统视图。
  - [complete] 迁移 dry-run 使用临时 PostgreSQL schema 或隔离测试库。
- [complete] 切片 3：业务 SQL 方言调整
  - [complete] `?` 占位符改为 PostgreSQL `$1/$2`。
  - [complete] `INSERT OR IGNORE` 改为 `ON CONFLICT DO NOTHING`。
  - [complete] 行类型和 `QueryBuilder` 泛型切换到 PostgreSQL。
- [complete] 切片 4：SQLite 到 PostgreSQL 数据同步
  - [complete] 新增 `db sync-sqlite --source data/blog.db --config config.yaml` 同步命令。
  - [complete] 实现从 SQLite 源读取全量表数据写入 PostgreSQL。
  - [complete] 同步后修复 PostgreSQL 自增序列。
  - [complete] 实际执行同步到本机 `blogweb`，核心数据已核对：users=7、categories=6、articles=7、likes=1、slug_history=1、bookmarks=1、author_follows=1。
- [in_progress] 切片 5：验证、提交和推送
  - [complete] `cargo fmt --check`
  - [complete] `cargo check`
  - [complete] `cargo test --no-run`
  - [complete] PostgreSQL 定向测试：`db_migration`、`sqlite_sync`、后台认证/读/写/用户测试。
  - [complete] `cargo test`
  - [complete] 阶段本地提交
  - [pending] 推送到远程：GitHub 443 连接失败，待网络恢复后执行 `git push`
