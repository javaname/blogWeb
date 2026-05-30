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

1. [in_progress] 写入邮箱注册设计文档与计划记录
2. [pending] 按 TDD 增加后端注册与验证码失败测试
3. [pending] 实现后端配置、迁移、邮件发送、注册接口和邮箱登录
4. [pending] 实现前端登录页注册表单和国际化文案
5. [pending] 运行 Go 与前端完整验证

# 新增阶段：后端 Rust 重构

目标：将当前 Go 后端逐步重构为 Rust 后端，在保持前端、SQLite 数据、配置文件、模板和公开 HTTP/MCP 行为兼容的前提下完成迁移。

当前状态：
- [complete] 盘点 Go 后端入口、路由、服务、数据表、配置和测试边界
- [complete] 确认迁移策略：一次性替换
- [complete] 写入并审查 Rust 迁移设计文档
- [complete] 生成 Go golden baseline，并按 TDD 创建 Rust 后端骨架和首批兼容性测试
- [complete] 切片 A 前半：配置默认合并、数据库迁移 check/apply、应用装配、`/healthz`
- [complete] 切片 A 后半：CLI `db check/migrate`、SQLite pool 装配、启动时 check-only 基础流程
- [in_progress] 切片 B：公开只读页面/API、模板渲染、文章列表、文章详情、分类页、静态资源
  - [complete] `GET /api/articles` 与 `GET /api/articles/:slug` 对齐当前 Go golden body
  - [complete] 分类筛选、keyword、隐藏草稿/未来发布、历史 slug 301
  - [complete] cursor 分页契约
  - [complete] 真实 Markdown renderer / sanitizer
  - [complete] 公开首页、文章页、分类页、assets/uploads 静态资源基础兼容
  - [pending] 完整 Tera 模板复刻、相关/评论展示、页面细节对齐
- [in_progress] 切片 C：后台认证与会话兼容
  - [complete] `POST /api/admin/login` 对齐 Go golden body 与 `admin_session` cookie 契约
  - [complete] `GET /api/admin/csrf-token` 未登录对齐 Go golden 401 body
  - [complete] 登录后会话可读取 `GET /api/admin/csrf-token` 与 `GET /api/admin/me`
  - [pending] Redis 会话存储、bcrypt 密码校验、logout、CSRF 写接口保护
- [in_progress] 切片 D：后台管理只读 API
  - [complete] `GET /api/admin/dashboard` 真实统计、活动和 30 天趋势
  - [complete] `GET /api/admin/settings` 返回公开运行时策略且不泄露密钥
  - [complete] `GET /api/admin/articles`、`/categories`、`/comments` 列表基础兼容
  - [pending] `GET /api/admin/articles/:id` 编辑详情、筛选/排序更多边界、完整 golden 覆盖
- [in_progress] 切片 E：后台写接口与 CSRF
  - [complete] 后台写接口缺失/错误 CSRF token 返回 Go 兼容 403 `csrf_invalid`
  - [complete] `POST /api/admin/categories` 基础创建
  - [complete] `POST /api/admin/articles` 基础创建和外部 http 封面拒绝
  - [complete] `PUT /api/admin/comments/:id/status` 基础审核状态更新
  - [pending] 文章编辑详情、文章更新/删除、分类更新/排序/删除、评论删除、设置更新、上传接口
- [in_progress] 切片 F：前台读者互动 API
  - [complete] `POST /api/articles/:slug/like` 与 `POST /api/likes/batch` 支持 `anonymous_id` cookie
  - [complete] `POST /api/articles/:slug/bookmark`
  - [complete] `POST /api/authors/:id/follow`
  - [complete] `POST /api/newsletter/subscribe`
  - [complete] `POST /api/articles/:slug/comments`
  - [pending] 读者互动速率限制、敏感词完整策略、前台页面评论展示完全复刻
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
  - [complete] MCP HTTP rate limit：read/write/publish/upload 分桶限流
  - [complete] 运行文档：README 与 MCP 客户端接入说明补充 Rust CLI 命令和行为差异
- [pending] 一次性迁移配置、数据库、认证会话、文章/分类/评论/互动、上传和 MCP 能力
- [pending] 更新启动文档与验证命令

# 新增阶段：Review 修复与后台原型落地

目标：先修复 review 指出的安全与正确性问题，再继续执行后台管理端 Stitch 原型落地。

当前状态：
- [complete] 修复 review 问题：配置密钥暴露、匿名访客 ID 不一致、MCP 草稿作者硬编码
- [complete] 运行后端定向测试和全量验证
- [complete] 按已批准规格实现后台管理端 Stitch 原型落地
- [complete] 运行前端 i18n、UI 和构建验证
