# 个人博客系统实现任务清单

> 基于 `2026-05-13-blog-design-v6.md` 拆解。`v5` 中未被 `v6` 修改的任务继续有效，`v6` 新增 MCP Server 相关实施任务。MCP 具体工具参数、错误码、权限矩阵与测试用例以 `2026-05-14-blog-mcp-implementation-spec.md` 为准。

## 一、总体实施顺序

建议按以下阶段推进，避免前后返工：

1. 基础设施与项目骨架
2. 数据模型、迁移与配置
3. 安全基线与中间件
4. 公开站内容与点赞系统
5. 管理后台认证与内容管理
6. 上传、静态资源与运维
7. MCP Server 基础能力
8. MCP 写工具、审计与安全
9. 联调、测试与上线准备

---

## 二、并行任务拆分（互不干扰）

以下拆分以“文件所有权”而不是“功能讨论范围”为准，目标是让多人并行时不改同一批文件。

### 2.1 拆分规则

- 每个任务包只修改自己的目录或文件，不跨包直接改动。
- `main.go`、配置结构、命令入口只由基础骨架包负责接线，其他任务包通过注册函数或接口对接。
- `migrations/` 与 `internal/model/` 只由数据层包负责，其他任务包只提字段和索引需求，不直接修改。
- `internal/mcp/` 按文件拆分所有权，读链路与写链路/安全链路分开，不共享同一文件。
- 联调与测试包默认只新增测试文件、脚本和说明文档，不回改业务实现文件；若发现阻塞问题，退回对应 owner 处理。

### 2.2 任务包

#### A 包：基础骨架与配置

- 对应阶段项：`1.1`、`2.1`、`2.4`、`2.6`、`7.2` 中命令入口接线部分
- 可修改范围：
  - `go.mod`
  - `main.go`
  - `config/`
  - 应用 bootstrap 与命令注册代码
- 不可修改范围：
  - `migrations/`
  - `internal/model/`
  - `client/`
  - `internal/mcp/` 的具体协议实现
- 交付结果：
  - 项目可启动
  - 配置结构冻结
  - `serve-web` / `serve-mcp` 命令入口可挂接

#### B 包：数据模型与迁移

- 对应阶段项：`2.2`、`2.3`、`2.5`
- 可修改范围：
  - `migrations/`
  - `internal/model/`
  - 数据访问层基础文件
- 不可修改范围：
  - handler
  - middleware
  - `client/`
  - `internal/mcp/`
- 交付结果：
  - `001_init.sql`、`002_mcp.sql` 可独立执行
  - 核心表和 MCP 表结构冻结
  - GORM 模型可供其他包复用

#### C 包：通用安全基线与中间件

- 对应阶段项：`3.1`、`3.2`、`3.3`、`3.4`、`3.5`
- 可修改范围：
  - `internal/middleware/`
  - Redis 限流与会话相关通用安全组件
  - 通用安全日志组件
- 不可修改范围：
  - 公开站 handler
  - 后台业务 handler
  - `client/`
  - `internal/mcp/`
- 交付结果：
  - Redis session、CSRF、响应头、登录限流、点赞限流能力可复用

#### D 包：公开站内容与点赞

- 对应阶段项：`4.1`、`4.2`、`4.3`、`4.4`、`4.5`、`4.6`
- 可修改范围：
  - 公开站 handler
  - `templates/`
  - 文章公开读取、slug、点赞相关 service 文件
- 不可修改范围：
  - 后台认证与管理接口
  - `client/`
  - 上传存储实现
  - `internal/mcp/`
- 交付结果：
  - 公开文章列表、详情、分类筛选、slug 跳转、点赞链路完成
  - 只读可见性规则稳定，可供 MCP 只读链路复用

#### E 包：后台认证与内容管理

- 对应阶段项：`1.2`、`5.1`、`5.2`、`5.3`、`5.4`
- 可修改范围：
  - `client/`
  - 后台登录与后台内容管理 handler
  - 后台文章、分类管理相关 service 文件
- 不可修改范围：
  - 公开站模板
  - 上传底层存储
  - `internal/mcp/`
- 交付结果：
  - 后台登录、退出、文章管理、分类管理、会话超时处理完成

#### F 包：上传、静态资源与运维

- 对应阶段项：`6.1`、`6.2`、`6.3`、`6.4`
- 可修改范围：
  - 上传处理链路
  - 静态资源托管配置
  - 备份恢复脚本
  - 运维与审计说明
- 不可修改范围：
  - 公开站业务查询逻辑
  - 后台 CRUD 逻辑
  - `client/`
  - `internal/mcp/`
- 交付结果：
  - 上传安全链路可用
  - 静态托管安全策略可用
  - 备份恢复流程可执行

#### G 包：MCP 只读链路

- 对应阶段项：`7.1`、`7.2`、`7.3`、`7.4`
- 可修改范围：
  - `internal/mcp/bootstrap.go`
  - `internal/mcp/server.go`
  - `internal/mcp/transport_stdio.go`
  - `internal/mcp/transport_http.go`
  - `internal/mcp/resources.go`
  - `internal/mcp/resource_templates.go`
  - `internal/mcp/tools_read.go`
  - `internal/mcp/prompts.go`
- 不可修改范围：
  - `internal/mcp/auth.go`
  - `internal/mcp/scopes.go`
  - `internal/mcp/tools_write.go`
  - `internal/mcp/schemas.go`
  - `internal/mcp/audit.go`
  - `main.go`
- 交付结果：
  - `stdio` / HTTP 基础 transport 可启动
  - resources、只读 tools、prompts 可用
  - 只读链路复用公开站可见性规则

#### H 包：MCP 写链路、鉴权与审计

- 对应阶段项：`8.1`、`8.2`、`8.3`、`8.4`、`8.5`、`8.6`
- 可修改范围：
  - `internal/mcp/auth.go`
  - `internal/mcp/scopes.go`
  - `internal/mcp/tools_write.go`
  - `internal/mcp/schemas.go`
  - `internal/mcp/audit.go`
  - MCP token CLI 实现文件
- 不可修改范围：
  - `internal/mcp/resources.go`
  - `internal/mcp/tools_read.go`
  - `internal/mcp/prompts.go`
  - `main.go`
  - `internal/model/`
- 交付结果：
  - token 签发/撤销
  - HTTP 鉴权、Origin 校验、scope 判定
  - 写工具、审计、限流、Prompt Injection 防护完成

#### I 包：测试、联调与上线准备

- 对应阶段项：`9.1`、`9.2`、`9.3`、`9.4`
- 可修改范围：
  - 单元测试文件
  - 集成测试脚本
  - 安全测试说明
  - 部署、备份、上线检查文档
- 不可修改范围：
  - 业务实现文件
  - 配置结构
  - MCP 协议契约
- 交付结果：
  - 自动化测试与上线检查清单完整
  - 联调问题清单可回流到对应 owner

### 2.3 推荐并行批次

为避免互相等待，建议按以下批次推进：

1. 第一批：`A`、`B`、`C`
2. 第二批：`D`、`E`、`F`
3. 第三批：`G`
4. 第四批：`H`
5. 第五批：`I`

依赖说明：

- `D`、`E` 依赖 `A`、`B`、`C`
- `F` 依赖 `A`、`B`
- `G` 依赖 `A`、`B`、`D`
- `H` 依赖 `A`、`B`、`C`、`F`、`G`
- `I` 依赖 `D`、`E`、`F`、`G`、`H` 基本完成

### 2.4 阶段清单与任务包的关系

以下“阶段任务”章节继续作为总排期清单使用；实际多人并行时，以本节任务包边界为准，不按阶段横向拆人。

---

## 三、阶段任务

### 阶段 1：基础设施与项目骨架

#### 1.1 初始化后端项目结构

- 创建目录结构：
  - `config/`
  - `internal/middleware/`
  - `internal/handler/`
  - `internal/model/`
  - `internal/service/`
  - `templates/`
  - `public/`
  - `migrations/`
- 初始化 `go.mod`
- 选定并引入核心依赖：
  - `gin`
  - `gorm`
  - `sqlite` driver
  - `gin-contrib/sessions`
  - `gin-contrib/sessions/redis`
  - `bcrypt`
  - `goldmark`
  - HTML sanitizer 库

完成标准：

- 项目可成功 `go build`
- 目录结构与设计文档一致

#### 1.2 初始化前端管理后台项目

- 创建 `client/`
- 初始化 Vite + React
- 接入 Semi Design
- 建立基础路由页面：
  - `Login`
  - `Dashboard`
  - `ArticleEdit`
  - `Categories`
- 建立统一 API 请求封装

完成标准：

- `npm run dev` 可启动
- 能打开后台页面骨架

---

### 阶段 2：数据模型、迁移与配置

#### 2.1 实现配置加载

- 实现 `config.yaml` 解析
- 支持配置项：
  - `server`
  - `database`
  - `redis`
  - `session`
  - `rate_limit`
  - `upload`
  - `admin`
  - `mcp`

完成标准：

- 启动时能正确读取配置
- 配置缺失时有明确错误信息

#### 2.2 实现数据库迁移

- 编写 `001_init.sql`
- 建表：
  - `users`
  - `categories`
  - `articles`
  - `likes`
  - `slug_history`
- 补约束与索引：
  - 唯一索引
  - 外键
  - `slug_history.article_id ON DELETE SET NULL`
  - `slug_history.old_slug` 唯一索引
  - `slug_history.article_id` 普通索引

完成标准：

- 新环境可一键初始化数据库
- 表结构与设计文档一致

#### 2.3 实现 GORM 模型与仓储层

- 为所有核心表建立模型
- 统一 `created_at` / `updated_at`
- 封装基础 CRUD

完成标准：

- 可通过单元测试完成基本增删改查

#### 2.4 初始化管理员账号

- 启动时检测 admin 是否存在
- 不存在则使用配置创建初始管理员
- 密码采用 bcrypt 哈希

完成标准：

- 新数据库首次启动后自动生成 admin 账号

#### 2.5 实现 MCP 数据结构迁移

- 编写 `002_mcp.sql`
- 建表：
  - `mcp_clients`
  - `mcp_audit_logs`
- 补约束与索引：
  - `mcp_clients.name` 唯一索引
  - `mcp_clients.token_hash` 唯一索引
  - `mcp_clients.is_enabled` 普通索引
  - `mcp_audit_logs.client_id` 普通索引
  - `mcp_audit_logs.created_at` 普通索引
  - `mcp_audit_logs.action_type` 普通索引

完成标准：

- `002_mcp.sql` 可在 `001_init.sql` 后独立执行
- MCP 表结构与 `v6` 设计一致
- 不存储明文 token

#### 2.6 实现 MCP 配置加载

- 在 `config.yaml` 支持 `mcp` 配置段：
  - `enabled`
  - `stdio_enabled`
  - `stdio_write_enabled`
  - `http_enabled`
  - `http_addr`
  - `http_path`
  - `auth_mode`
  - `require_origin_check`
  - `allowed_origins`
  - `stateless_http`
  - `protocol_versions`
  - `rate_limit`

完成标准：

- 配置缺失时有安全默认值
- `stdio_write_enabled` 默认 `false`
- `http_enabled` 默认 `false`
- `stateless_http` 默认 `true`

---

### 阶段 3：安全基线与中间件

#### 3.1 实现 Redis 会话与 CSRF

- 接入 Redis session store
- 登录成功后重新生成 session id
- 为 session 生成 CSRF token
- 实现 `GET /api/admin/csrf-token`

完成标准：

- 登录后能拿到 session 和 CSRF token
- 写接口缺少 token 时拒绝访问

#### 3.2 实现认证与授权中间件

- `auth` 中间件：校验已登录
- `admin_role` 中间件：校验 `role=admin`
- 后台接口统一挂载

完成标准：

- 未登录返回 `401`
- 非 admin 返回 `403`

#### 3.3 实现安全响应头中间件

- 设置：
  - `Content-Security-Policy`
  - `X-Content-Type-Options: nosniff`
  - `Referrer-Policy`
  - `X-Frame-Options` 或 `frame-ancestors`

完成标准：

- 所有公开页、后台页、静态资源响应头符合设计

#### 3.4 实现登录限流与冷却

- Redis 计数器实现：
  - 单 IP 时间窗口限流
  - 单用户名失败次数冷却
- 登录成功后清空失败计数
- 命中时返回 `429`

完成标准：

- 重复错误密码会触发限流或冷却
- 安全日志可记录相关事件

#### 3.5 实现点赞限流

- 单 IP 限流
- 单文章单 IP 状态变更频率限制
- 命中时返回 `429`

完成标准：

- 点赞接口可阻断明显刷量行为

---

### 阶段 4：公开站内容与点赞系统

#### 4.1 实现 Markdown 渲染与 Sanitizer

- Markdown 转 HTML
- 渲染结果通过白名单 sanitizer 清洗
- 禁止危险标签、属性、协议

完成标准：

- 常规 Markdown 正常渲染
- 注入型 payload 被清除

#### 4.2 实现 slug 规则与历史跳转

- 自动生成 slug
- 避开 `articles.slug` 与 `slug_history.old_slug`
- 更新标题时写入 `slug_history`
- 路由解析顺序：
  - 当前 slug
  - 历史 slug
- 满足条件时 `301`，否则 `404`

完成标准：

- 标题修改后旧链接行为符合文档

#### 4.3 实现公开文章查询

- 只返回：
  - `status='published'`
  - `published_at <= now()`
- 实现：
  - 首页列表
  - 分类筛选
  - 文章详情

完成标准：

- 草稿与未来发布时间文章不会公开可见

#### 4.4 实现 cursor 分页

- 排序键：
  - `is_pinned DESC`
  - `published_at DESC`
  - `id DESC`
- 生成和解析 `next_cursor`

完成标准：

- 连续翻页不重复、不跳项

#### 4.5 实现匿名身份体系

- 下发 `anonymous_id` cookie
- 同步 `localStorage`
- 兼容 cookie 不可用场景
- 模板首屏可基于 cookie 渲染详情页点赞态

完成标准：

- 正常浏览器下首屏点赞状态可识别
- cookie 禁用场景下页面可退化运行

#### 4.6 实现点赞功能

- `POST /api/articles/:slug/like`
- `POST /api/likes/batch`
- 幂等处理
- 点赞数统计
- 异常请求处理

完成标准：

- `like` / `unlike` 行为与设计一致
- 列表页和详情页都能获取点赞状态

---

### 阶段 5：管理后台认证与内容管理

#### 5.1 实现后台登录/退出

- 登录接口
- 退出接口
- 会话生成、销毁
- 前端登录页联调

完成标准：

- 可登录后台
- 会话过期或登出后自动失效

#### 5.2 实现文章管理

- 后台文章列表：
  - 分页
  - 状态筛选
  - 分类筛选
  - 关键词搜索
  - 多字段排序
- 文章详情接口
- 新建文章
- 更新文章
- 删除文章

完成标准：

- 后台可完整管理文章生命周期

#### 5.3 实现分类管理

- 分类列表
- 新建分类
- 更新分类
- 删除分类
- 批量排序分类
- 删除时校验“存在已发布文章则禁止删除”

完成标准：

- 分类排序、删除规则与文档一致

#### 5.4 实现后台会话超时与前端处理

- 空闲超时
- 绝对超时
- 前端收到 `401` / `403` 的统一处理

完成标准：

- 后台前端能正确处理会话失效与权限不足

---

### 阶段 6：上传、静态资源与运维

#### 6.1 实现上传安全链路

- MIME 魔数校验
- 禁止 `svg`
- 重建扩展名
- 推荐重编码
- 存储到 `public/uploads/YYYY/MM/`

完成标准：

- 非法类型无法上传
- 返回路径格式稳定

#### 6.2 配置静态资源安全托管

- 上传目录只做静态文件
- 禁止脚本执行
- 返回正确 `Content-Type`
- 返回 `nosniff`

完成标准：

- 上传文件可访问，但不能作为脚本执行载体

#### 6.3 配置日志与审计

- 登录成功/失败日志
- 冷却与限流日志
- 上传失败日志
- 后台关键写操作日志

完成标准：

- 可用于排查安全与业务问题

#### 6.4 完成备份与恢复脚本

- SQLite `.backup`
- Redis 持久化策略确认
- 上传目录备份

完成标准：

- 能执行一次完整备份
- 能说明恢复步骤

---

### 阶段 7：MCP Server 基础能力

本阶段执行前需对齐 `2026-05-14-blog-mcp-implementation-spec.md` 中的 resources、只读 tools、prompts 与传输模式要求。

#### 7.1 初始化 MCP 项目结构

- 新增 `internal/mcp/`
- 建立基础文件：
  - `bootstrap.go`
  - `server.go`
  - `transport_stdio.go`
  - `transport_http.go`
  - `auth.go`
  - `scopes.go`
  - `resources.go`
  - `resource_templates.go`
  - `tools_read.go`
  - `tools_write.go`
  - `prompts.go`
  - `schemas.go`
  - `audit.go`
- 新增模型：
  - `internal/model/mcp_client.go`
  - `internal/model/mcp_audit_log.go`

完成标准：

- `go build` 通过
- MCP 目录仅处理协议适配，不直接承载业务核心逻辑

#### 7.2 实现 MCP 启动模式

- 支持命令：
  - `blogWeb serve-web`
  - `blogWeb serve-mcp --transport=stdio`
  - `blogWeb serve-mcp --transport=http`
- `stdio` 模式：
  - `stdout` 仅输出 MCP 协议消息
  - 日志输出到 `stderr`
- HTTP 模式：
  - 默认绑定 `127.0.0.1:3001`
  - 默认路径 `/mcp`
  - 默认无状态 POST + JSON 响应
  - 校验 `Content-Type: application/json`
  - 校验 `Accept` 至少包含 `application/json`
  - 支持 `MCP-Protocol-Version`
  - JSON-RPC notification / response 被接受后返回 `202 Accepted`
  - 未启用 SSE 时 GET 返回 `405`

完成标准：

- 本地可分别启动 Web 与 MCP 模式
- `stdio` 模式不会污染协议输出
- HTTP MCP 端点可完成基础初始化或工具调用

#### 7.3 实现 MCP resources

- 实现资源：
  - `blog://site/meta`
  - `blog://categories`
  - `blog://articles/{slug}`
  - `blog://drafts/{id}`
  - `blog://categories/{slug}/articles`
- 复用现有 service：
  - 公开文章只返回 `published` 且已到发布时间的文章
  - 草稿资源要求 `blog.draft.write`
  - 历史 slug 规则沿用设计文档

完成标准：

- 只读资源不会泄漏草稿和未来文章
- 草稿资源无权限时拒绝访问
- 返回内容符合 sanitizer 安全要求

#### 7.4 实现 MCP 只读工具与 prompts

- 只读工具：
  - `list_articles`
  - `get_article`
  - `list_categories`
  - `preview_markdown`
- Prompts：
  - `draft_article_from_outline`
  - `seo_review_article`
  - `rewrite_article_summary`

完成标准：

- 只读工具与公开 API 可见性一致
- `preview_markdown` 不落库，并复用 Markdown sanitizer
- prompt 输出不直接触发持久化写入

---

### 阶段 8：MCP 写工具、审计与安全

本阶段执行前需对齐 `2026-05-14-blog-mcp-implementation-spec.md` 中的写工具参数、scope 权限矩阵、错误码、审计字段与安全测试用例。

#### 8.1 实现 MCP 客户端凭证 CLI

- 实现命令：
  - `blogWeb mcp issue-token --name <client-name> --scopes <scopes>`
  - `blogWeb mcp revoke-token --name <client-name>`
- token 签发规则：
  - 高熵随机串
  - 明文只展示一次
  - 数据库存储哈希，推荐 `HMAC-SHA256(server_secret, token)` 或同等强度方案
  - token 校验使用常量时间比较
  - 可按客户端禁用

完成标准：

- 可签发只读 token 和写权限 token
- 撤销后 token 立即不可用
- 数据库不出现明文 token

#### 8.2 实现 MCP HTTP 鉴权与 Origin 校验

- HTTP 模式必须校验：
  - `Authorization: Bearer <token>`
  - `mcp_clients.is_enabled`
  - scope 覆盖本次能力
  - `Origin` 是否在白名单
- 401 响应返回 `WWW-Authenticate`
- scope 不足时返回 `WWW-Authenticate: Bearer error="insufficient_scope"`
- 首版使用私有预注册 token；公网第三方客户端场景需另补 OAuth Protected Resource Metadata
- 后台 session cookie 不参与 MCP 鉴权

完成标准：

- 缺少 token 返回未授权错误
- token 无效或已撤销时拒绝
- scope 不足时拒绝
- 非法 Origin 时拒绝
- 不支持的 `MCP-Protocol-Version` 被拒绝

#### 8.3 实现 MCP 写工具

- 实现写工具：
  - `create_article_draft`
  - `update_article`
  - `publish_article`
  - `unpublish_article`
  - `upload_image`
  - `create_category`
  - `update_category`
- 明确不实现：
  - `delete_article`
  - `delete_category`
  - 任意 SQL
  - 任意文件系统访问
  - Shell 执行

完成标准：

- 写工具全部复用 `service` 层
- 发布规则、slug 规则、上传规则与 Web 后台一致
- 删除类能力不存在于工具列表

#### 8.4 实现 MCP 参数 schema 与白名单校验

- 对所有 tools / prompts / resources 参数做严格校验：
  - slug 字符集
  - title 长度
  - Markdown 最大长度
  - 批量数组最大长度
  - 上传大小与 MIME
  - `cover_image` 只能引用 `/uploads/YYYY/MM/{uuid}.{ext}` 格式站内资源
  - 未知字段拒绝或忽略

完成标准：

- 非法参数不会进入 service 层
- 大 payload 被拦截
- 未知字段无法隐式扩权

#### 8.5 实现 MCP 审计与限流

- 审计事件：
  - resource 读取
  - tool 调用
  - prompt 获取
  - 鉴权失败
  - scope 不足
  - 限流命中
  - 服务端错误
- Redis 限流：
  - `mcp_read_rate:{client_id}`
  - `mcp_write_rate:{client_id}`
  - `mcp_upload_rate:{client_id}`
- 审计脱敏：
  - 不记录明文 token
  - 不记录完整正文
  - 不记录图片 base64

完成标准：

- 可按客户端、工具名、时间窗口查询审计日志
- 高频调用会触发限流
- 审计日志不包含敏感明文

#### 8.6 实现 Prompt Injection 防护约束

- 文章正文、分类名称、上传文件名均视为不可信输入
- Prompt 模板将文章内容标记为待分析数据
- 写操作必须由显式工具调用触发
- 不允许资源正文隐式驱动高权限工具参数

完成标准：

- prompt 输出不会自动落库
- 资源内容不会绕过显式工具调用触发发布或上传

---

### 阶段 9：联调、测试与上线准备

#### 9.1 单元测试

- slug 生成逻辑
- slug 历史跳转逻辑
- Markdown sanitizer
- 登录限流
- 点赞限流
- 文章可见性判断
- MCP scope 判定
- MCP 参数 schema 校验
- MCP token 哈希与校验
- MCP 审计脱敏

完成标准：

- 核心规则均有自动化测试覆盖

#### 9.2 集成测试

- 登录流程
- 后台文章 CRUD
- 分类删除规则
- 点赞流程
- 上传流程
- 历史 slug 跳转
- MCP stdio 只读调用
- MCP HTTP token 鉴权
- MCP 写工具创建草稿、更新、发布、上传

完成标准：

- 关键链路可跑通

#### 9.3 安全测试

- XSS payload 测试
- SVG / 伪装图片上传测试
- 登录爆破模拟
- 点赞刷量模拟
- CSRF token 缺失测试
- MCP 缺少 token / 无效 token / scope 不足测试
- MCP 鉴权失败 `WWW-Authenticate` 响应头测试
- MCP 非法 Origin 测试
- MCP 协议头 `Accept` / `MCP-Protocol-Version` 测试
- MCP 上传伪装文件测试
- MCP `cover_image` 外部 URL / 路径穿越测试
- MCP Prompt Injection 场景测试

完成标准：

- 关键安全场景符合预期拦截

#### 9.4 上线检查清单

- HTTPS 已启用
- Redis 持久化已配置
- 安全响应头已生效
- 管理员初始密码已修改
- 上传目录权限已确认
- 备份策略已验证
- MCP HTTP 默认未公网暴露，或已配置 HTTPS、Origin 白名单、Bearer Token、反向代理 ACL
- MCP 写权限 token 已按最小权限签发
- MCP 审计日志已启用

完成标准：

- 满足上线前最低安全要求

---

## 四、建议交付物

开发过程中建议同步产出：

- `README.md`
- `config.example.yaml`
- 数据库迁移脚本
- 安全测试说明
- 部署说明
- 备份恢复说明
- MCP 客户端接入说明
- MCP token 签发与撤销说明
- MCP 安全测试说明

---

## 五、里程碑建议

### M1：后端基础可启动

- 配置、迁移、Redis、session、admin 初始化完成

### M2：公开站可用

- 文章列表、详情、slug 跳转、点赞完成

### M3：后台可用

- 登录、文章管理、分类管理、上传完成

### M4：安全与运维完成

- 限流、sanitizer、响应头、日志、备份、测试完成

### M5：MCP 只读能力完成

- `stdio` 模式可被本地 AI 客户端发现并调用
- MCP resources 与只读 tools 完成
- 只读权限不泄漏草稿和未来文章

### M6：MCP 写能力与安全完成

- MCP token、scope、Origin 校验、写工具、审计、限流完成
- 写工具与后台业务规则一致
- MCP 安全测试通过
