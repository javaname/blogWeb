# 个人博客系统设计规范（第六版）

> 修订于 2026-05-13，基于第五版新增 MCP 集成设计，并补齐 AI 接入场景下的权限、安全与运维边界。

---

## 一、版本说明

- `v6` 为当前主审设计文档。
- `v5` 中未被 `v6` 明确修改的内容继续有效，主要包括：
  - 博客核心业务模型：`users`、`categories`、`articles`、`likes`、`slug_history`
  - 公开站点页面与后台管理页面设计
  - 现有 HTTP API 契约
  - Markdown 渲染安全、上传安全、点赞防刷、登录防暴破、安全响应头、Redis 会话等既有安全基线
- 当 `v5` 与 `v6` 冲突时，以 `v6` 为准。

---

## 二、本版目标

### 2.1 设计目标

- 在不推翻 `v5` 架构的前提下，为当前博客项目新增 MCP 能力。
- 使项目能够作为 **MCP Server** 被 AI 客户端接入，安全暴露文章读取、草稿编辑、发布、上传等受控能力。
- 保持后端单技术栈：继续使用 Go，不新增 Python/Node 服务端。
- 所有 MCP 写能力必须复用博客现有 `service` 层，不能绕过既有业务规则和安全校验。

### 2.2 非目标

- `v6` 不将当前博客实现为 MCP Client。
- `v6` 不提供任意 SQL、任意文件系统访问、任意 Shell 执行能力。
- `v6` 第一阶段默认 **不暴露高风险删除能力**：
  - 不提供 `delete_article`
  - 不提供 `delete_category`
  - 不提供任意批量数据导出
- `v6` 不改写 `v5` 的站点公开 API，不以 MCP 替代前台或后台 HTTP API。

---

## 三、总体架构调整

`v5` 的公开站点、后台 API、后台 SPA 三层结构保持不变；`v6` 新增一个协议接入层：`MCP Server`。

```text
AI Client / IDE / Agent
        |
        | MCP (stdio / Streamable HTTP)
        v
   MCP Server Adapter
        |
        | 参数校验 / 权限校验 / 审计 / 限流
        v
     Service Layer
        |
        | 统一业务规则
        v
 SQLite / Redis / Upload Storage

Browser
   |                    Admin SPA
   | HTTP               | HTTP
   v                    v
Public Handlers      Admin Handlers
        \              /
         \            /
          \          /
            Service Layer
```

### 3.1 架构原则

- MCP 只是一层协议适配，不是新的业务核心。
- MCP 层必须调用现有 `service`，不能直接读写数据库。
- MCP 层不能绕过：
  - Markdown sanitizer
  - 上传 MIME 校验与重编码
  - slug 保留规则
  - 发布状态规则
  - 点赞、登录、上传等限流和审计机制
- Web、Admin、MCP 三种入口的业务结果必须保持一致。

---

## 四、MCP 角色选择与技术栈

### 4.1 角色选择

当前项目在 `v6` 中优先实现为 **MCP Server**，理由如下：

- 该项目本身拥有文章、分类、草稿、上传等内部业务能力，天然适合对外暴露为受控工具与资源。
- 现阶段业务重点是“让 AI 使用博客能力”，而不是“让博客消费外部 MCP 服务”。
- 以 Server 方式接入，对当前代码库入侵最小，最容易复用既有业务层。

### 4.2 技术栈补充

| 层级 | 选型 | 说明 |
|------|------|------|
| MCP 协议角色 | MCP Server | 对外暴露 `resources` / `tools` / `prompts` |
| MCP 传输 | `stdio` + `Streamable HTTP` | 本地集成优先 `stdio`，服务化接入使用 `Streamable HTTP` |
| MCP SDK | Go 官方 SDK 或等价官方兼容实现 | 保持与现有 Go 技术栈一致，降低手写协议出错率 |
| MCP 鉴权 | `Bearer Token` + scope | 仅用于 HTTP 传输；不复用后台 session |
| MCP 审计 | 结构化日志 + 审计表 | 记录资源读取、工具调用、拒绝与异常 |
| MCP 限流 | Redis | 与 `v5` 现有 Redis 基线一致 |

### 4.3 技术栈一致性约束

- 不新增第二套服务端语言。
- 不引入独立 Python MCP Sidecar。
- 不将后台 React 前端直接嵌入 MCP 执行链路。
- MCP 与既有 HTTP API 共用相同业务模型、数据库、Redis、上传目录和安全策略。

---

## 五、运行模式与传输设计

### 5.1 启动模式

单二进制继续保留，新增 MCP 启动模式：

```text
blogWeb serve-web
blogWeb serve-mcp --transport=stdio
blogWeb serve-mcp --transport=http
```

说明：

- `serve-web`：运行博客 Web 服务与后台服务，对应 `v5` 主流程
- `serve-mcp --transport=stdio`：供本地 AI 客户端以子进程方式拉起
- `serve-mcp --transport=http`：运行独立 MCP HTTP 服务

### 5.2 `stdio` 传输

适用场景：

- 本地开发
- IDE / 桌面 AI 客户端
- 单机助手接入

约束：

- 只通过 `stdin/stdout` 传输 MCP JSON-RPC 消息
- `stdout` 禁止输出任何非 MCP 消息
- 日志仅允许写入 `stderr`
- 本地 `stdio` 默认不开启网络监听

默认权限策略：

- 默认开放只读能力
- 写能力通过配置显式开启：`mcp.stdio_write_enabled=true`

### 5.3 `Streamable HTTP` 传输

适用场景：

- 自托管 AI 客户端
- 团队内受控服务接入
- 需要长期在线的 MCP Server

基线设计：

- HTTP MCP 服务与公开博客站点 **逻辑上隔离**
- 默认使用独立监听地址，而不是直接挂到公开站点路由树
- 推荐默认监听：`127.0.0.1:3001`
- 推荐 MCP 路径：`/mcp`
- 若需公网暴露，必须经反向代理、HTTPS、来源校验与令牌鉴权后再开放
- 首版实现默认采用 **无状态（stateless）** 模式
- 首版实现默认采用“单次 POST 请求 -> 单次 JSON 响应”模式，不依赖 SSE 推送链路
- POST 请求必须使用 JSON-RPC 消息体，并携带 `Accept` 头，至少包含 `application/json`；为兼容标准客户端，建议同时包含 `application/json, text/event-stream`
- POST 请求若是 JSON-RPC request，服务端返回 `Content-Type: application/json` 的单个 JSON-RPC 响应；若是 notification / response，服务端接受后返回 `202 Accepted` 且无响应体
- 如客户端发起 GET 到 MCP 端点，而服务端未启用 SSE 监听，可返回 `405 Method Not Allowed`
- HTTP 请求应支持 `MCP-Protocol-Version` 头；不支持的协议版本返回 `400 Bad Request`

### 5.4 HTTP Session 语义

`v6` 默认 **不依赖协议级 Session**。理由：

- 当前博客场景以资源读取、草稿编辑、发布、上传为主，不需要复杂的服务端主动推送
- 无状态实现更容易部署、扩缩容和审计
- MCP 规范正在持续演进，减少对协议级 session 语义的耦合更稳妥

兼容策略：

- 若所选 Go MCP SDK 在特定版本下仍支持或要求 `Mcp-Session-Id`，可作为 **兼容模式** 启用
- 即使启用兼容模式，也不得把后台登录 session 与 MCP session 打通
- 所有真正影响业务一致性的上下文，仍应通过显式参数、资源 URI 或服务端签发的业务级 handle 传递，而不是依赖隐式协议会话

因此，`v6` 不把 Redis 中的 MCP session 作为正式业务依赖项。

---

## 六、数据结构与 Redis 扩展

在 `v5` 基础上新增以下数据结构。

### 6.1 `mcp_clients` — MCP 客户端凭证表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | 主键 |
| name | TEXT | UNIQUE NOT NULL | 客户端名称，如 `chatgpt-prod` |
| token_hash | TEXT | UNIQUE NOT NULL | 访问令牌哈希，不存明文 |
| scopes | TEXT | NOT NULL | scope 列表，JSON 数组或规范化字符串 |
| transport | TEXT | NOT NULL DEFAULT 'http' | `stdio` / `http` / `both` |
| is_enabled | INTEGER | NOT NULL DEFAULT 1 | 是否启用 |
| created_by | INTEGER | FK → users.id, NULL | 由哪个后台用户创建 |
| last_used_at | DATETIME | NULL | 最近使用时间 |
| created_at | DATETIME | NOT NULL | 创建时间 |
| updated_at | DATETIME | NOT NULL | 更新时间 |

设计要求：

- 令牌必须为高熵随机串。
- 数据库存储 token 哈希值，不保存明文 token；推荐使用 `HMAC-SHA256(server_secret, token)` 或同等强度方案，并使用常量时间比较。
- 明文 token 仅在“签发当次”显示一次。
- 禁止将后台密码、session secret 直接当作 MCP token 使用。
- 不硬删除客户端凭证，撤销时将 `is_enabled` 置为 `0`，以保留审计关联。

### 6.2 `mcp_audit_logs` — MCP 审计日志表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | 主键 |
| client_id | INTEGER | FK → mcp_clients.id, NULL | 来源客户端 |
| transport | TEXT | NOT NULL | `stdio` / `http` |
| action_type | TEXT | NOT NULL | `resource_read` / `tool_call` / `prompt_get` |
| target | TEXT | NOT NULL | 资源 URI 或工具名 / prompt 名 |
| scope | TEXT | DEFAULT '' | 本次命中的授权 scope |
| status | TEXT | NOT NULL | `success` / `denied` / `rate_limited` / `error` |
| request_id | TEXT | DEFAULT '' | 请求链路标识 |
| actor_ip | TEXT | DEFAULT '' | HTTP 场景记录来源 IP |
| error_code | TEXT | DEFAULT '' | 错误码 |
| payload_digest | TEXT | DEFAULT '' | 参数摘要，不存完整敏感正文 |
| created_at | DATETIME | NOT NULL | 事件时间 |

设计要求：

- 审计表不落明文 token。
- 审计表不落完整文章正文、图片 base64、密码等敏感数据。
- 大 payload 只记录摘要或长度信息。

### 6.3 Redis Key 扩展

| 用途 | Key 格式 | 说明 |
|------|----------|------|
| MCP 读限流 | `mcp_read_rate:{client_id}` | 只读能力限流 |
| MCP 写限流 | `mcp_write_rate:{client_id}` | 写工具限流 |
| MCP 上传限流 | `mcp_upload_rate:{client_id}` | 上传工具限流 |
| MCP 防重放 | `mcp_nonce:{client_id}:{nonce}` | 可选，请求防重放 |
| MCP 业务句柄 | `mcp_handle:{handle_id}` | 可选，跨调用临时句柄，非协议级 session |

---

## 七、能力模型设计

`v6` 将 MCP 能力拆成三类：`resources`、`tools`、`prompts`。

### 7.1 能力开放原则

- 优先开放高价值低风险的只读能力
- 写能力默认最小化，按 scope 单独授权
- 删除类、高破坏性类操作不在首版开放
- prompt 只生成建议与模板，不直接持久化业务数据

### 7.2 Resources 设计

Resources 用于向 AI 客户端暴露结构化上下文，默认只读。

| URI / 模板 | scope | 说明 |
|------------|-------|------|
| `blog://site/meta` | `blog.read` | 站点标题、描述、基础配置摘要 |
| `blog://categories` | `blog.category.read` | 分类列表与排序 |
| `blog://articles/{slug}` | `blog.read` | 已发布文章详情，返回清洗后的 HTML 与元数据 |
| `blog://drafts/{id}` | `blog.draft.write` | 草稿原始 Markdown 与后台元数据 |
| `blog://categories/{slug}/articles` | `blog.read` | 某分类下已发布文章摘要列表 |

资源约束：

- `blog://articles/{slug}` 只返回公开可见文章
- 草稿资源必须要求写权限或专门后台权限
- 历史 slug 解析规则沿用 `v5`
- 所有文章正文输出必须继续经过 sanitizer 或使用受控原始 Markdown 字段

### 7.3 Tools 设计

Tools 用于执行查询或受控写操作。

#### 只读工具

| 工具名 | scope | 说明 |
|--------|-------|------|
| `list_articles` | `blog.read` | 查询已发布文章列表，支持 `cursor/category/limit` |
| `get_article` | `blog.read` | 获取单篇公开文章详情 |
| `list_categories` | `blog.category.read` | 获取分类列表 |
| `preview_markdown` | `blog.draft.write` | 将 Markdown 预览为已清洗 HTML，不落库 |

#### 写工具

| 工具名 | scope | 说明 |
|--------|-------|------|
| `create_article_draft` | `blog.draft.write` | 创建草稿文章 |
| `update_article` | `blog.draft.write` | 更新文章标题、正文、分类、封面、置顶等 |
| `publish_article` | `blog.publish` | 将草稿发布或调整发布时间 |
| `unpublish_article` | `blog.publish` | 将已发布文章切回草稿 |
| `upload_image` | `blog.upload` | 上传文章图片，复用 `v5` 上传安全规则 |
| `create_category` | `blog.category.write` | 创建分类 |
| `update_category` | `blog.category.write` | 修改分类名称、slug、排序 |

明确不开放：

- `delete_article`
- `delete_category`
- 任意 SQL 查询
- 任意本地文件读写
- 服务器命令执行

### 7.4 Prompts 设计

Prompts 用于提供结构化写作模板，不直接写库。

| Prompt 名 | 说明 | 推荐参数 |
|-----------|------|----------|
| `draft_article_from_outline` | 根据大纲生成博客草稿提示词 | `title`, `outline`, `audience`, `tone` |
| `seo_review_article` | 对文章进行 SEO 检查与建议 | `title`, `content`, `keywords` |
| `rewrite_article_summary` | 重写摘要或导语 | `title`, `content`, `target_length` |

Prompt 约束：

- prompt 输出是“建议内容”或“对话模板”，不是自动落库操作
- 真正落库必须再次调用写工具
- prompt 中嵌入的文章正文视为“待处理内容”，不能被当作可信系统指令

### 7.5 第一阶段不实现的 MCP 能力

- `sampling`
- `roots`
- 资源订阅推送
- 删除类写工具
- MCP Apps / 会话内 UI 组件能力

说明：以上能力不是协议冲突，而是本项目首版设计中主动收缩范围。

---

## 八、权限模型与安全边界

### 8.1 Scope 设计

定义以下 MCP scope：

| scope | 权限说明 |
|-------|----------|
| `blog.read` | 读取公开文章与站点元数据 |
| `blog.category.read` | 读取分类信息 |
| `blog.draft.write` | 创建与编辑草稿、读取草稿、预览 Markdown |
| `blog.publish` | 发布、撤回发布、修改发布时间 |
| `blog.upload` | 上传图片 |
| `blog.category.write` | 新建、修改分类与分类排序 |

规则：

- scope 之间不自动继承
- `blog.publish` 不隐含 `blog.draft.write`
- `blog.upload` 不隐含文章写权限
- 客户端令牌必须最小授权

### 8.2 鉴权策略

#### `stdio`

- 视为“本地受控启动”
- 默认仅启用只读工具与资源
- 若需要写能力，必须显式配置 `mcp.stdio_write_enabled=true`

#### `Streamable HTTP`

- 必须使用 `Authorization: Bearer <token>`
- token 对应 `mcp_clients`
- 必须校验客户端是否启用、scope 是否覆盖本次能力
- 不能复用后台 session cookie 作为 MCP 认证
- 首版 Bearer Token 为私有预注册令牌模式，只面向本地或受控客户端
- 若未来要向通用第三方 MCP 客户端或公网 OAuth 场景开放，必须补充 OAuth Protected Resource Metadata、`WWW-Authenticate` challenge、scope challenge 与 token audience 校验

### 8.3 来源校验与网络边界

针对 HTTP 传输，必须：

- 校验请求中的 `Origin`
- 默认只绑定 `127.0.0.1`
- 非本地暴露时强制 HTTPS
- 由反向代理明确限制来源、方法、头部与请求体大小

拒绝策略：

- `Origin` 存在但不在白名单：拒绝
- 公网暴露或浏览器客户端场景下缺少 `Origin`：拒绝
- token 无效或禁用：拒绝
- scope 不足：拒绝

鉴权错误响应要求：

- 401 应返回 `WWW-Authenticate: Bearer ...`，并指向 MCP Protected Resource Metadata 或私有令牌签发说明
- 403 scope 不足时，应返回 `WWW-Authenticate: Bearer error="insufficient_scope", scope="..."`，明确本次调用所需 scope

### 8.4 参数校验

所有 tools / prompts / resources 参数都必须做白名单校验：

- slug、分类 slug 仅允许合法字符
- 标题、摘要、分类名称限制长度
- Markdown 正文限制最大长度
- 批量查询限制最大数组长度
- 上传图片限制 MIME、大小、扩展名与重编码策略
- `cover_image` 等站内路径只能引用 `/uploads/` 下的已存在资源，禁止外部 URL、绝对磁盘路径和 `..` 路径穿越
- 任何未知字段默认拒绝或忽略，避免隐式扩权

### 8.5 写能力安全规则

MCP 写工具必须复用 `v5` 已定义的全部业务规则：

- `create_article_draft` 默认创建 `draft`
- `publish_article` 必须遵守 `published_at` 规则
- `update_article` 若标题变化导致 slug 更新，必须写入 `slug_history`
- `upload_image` 必须沿用：
  - MIME 魔数校验
  - 禁止 SVG
  - 重建扩展名
  - 推荐重编码
  - `nosniff`
- `preview_markdown` 与 `get_article` 一样，必须使用安全渲染链路

### 8.6 Prompt Injection / Resource Injection 防护

MCP 引入后，文章内容本身可能成为 AI 上下文输入，因此新增以下规则：

- 文章正文、分类描述、上传文件名都视为 **不可信内容**
- prompt 模板中引用文章内容时，必须以“待分析数据”方式嵌入，而不是系统级指令
- 不允许将资源正文直接拼接成高权限工具的隐式参数
- 发布、上传、分类修改等写操作必须由显式 tool 调用触发
- 对高风险工具建议要求客户端侧人工确认

### 8.7 审计与限流

MCP 调用必须进入审计链路：

- 记录成功调用
- 记录鉴权失败
- 记录 scope 不足
- 记录限流命中
- 记录服务端错误

推荐限流：

| 类型 | 推荐值 |
|------|--------|
| 读资源/只读工具 | 单 client 每分钟 120 次 |
| 写工具 | 单 client 每分钟 30 次 |
| 发布工具 | 单 client 每 10 分钟 10 次 |
| 上传工具 | 单 client 每 10 分钟 10 次 |

---

## 九、与现有业务层的映射关系

### 9.1 复用原则

- MCP 不新增第二套文章服务
- MCP 不复制后台 API 逻辑
- MCP 通过参数 DTO 转换后调用 `service` 层

### 9.2 映射示意

| MCP 能力 | 复用服务 |
|----------|----------|
| `list_articles` | `ArticleService.ListPublished` |
| `get_article` | `ArticleService.GetPublishedBySlug` |
| `create_article_draft` | `ArticleService.Create` |
| `update_article` | `ArticleService.Update` |
| `publish_article` / `unpublish_article` | `ArticleService.UpdateStatus` |
| `list_categories` | `CategoryService.List` |
| `create_category` / `update_category` | `CategoryService.Create/Update` |
| `upload_image` | `UploadService.ValidateAndStore` |
| `preview_markdown` | `SanitizerService.RenderSafeHTML` |

### 9.3 一致性要求

- 同一篇文章经后台 UI 编辑和经 MCP 工具编辑，数据库结果必须一致
- 同一资源经公开 API 返回和经 MCP 资源返回，公开可见性规则必须一致
- 任何新业务规则必须先落到 `service` 层，再被 HTTP 与 MCP 共同复用

---

## 十、项目结构调整

在 `v5` 结构基础上新增 `internal/mcp/` 目录：

```text
blogWeb/
├── main.go
├── internal/
│   ├── mcp/
│   │   ├── bootstrap.go
│   │   ├── server.go
│   │   ├── transport_stdio.go
│   │   ├── transport_http.go
│   │   ├── auth.go
│   │   ├── scopes.go
│   │   ├── resources.go
│   │   ├── resource_templates.go
│   │   ├── tools_read.go
│   │   ├── tools_write.go
│   │   ├── prompts.go
│   │   ├── schemas.go
│   │   └── audit.go
│   ├── middleware/
│   ├── handler/
│   ├── model/
│   └── service/
└── migrations/
    ├── 001_init.sql
    └── 002_mcp.sql
```

约束：

- `internal/mcp/` 只处理协议适配，不应沉淀业务核心逻辑
- `schemas.go` 负责 tool/prompt 参数 schema
- `audit.go` 负责 MCP 调用审计
- 数据模型新增：
  - `model/mcp_client.go`
  - `model/mcp_audit_log.go`

---

## 十一、配置设计

在 `config.yaml` 中新增 `mcp` 段：

```yaml
mcp:
  enabled: true

  stdio_enabled: true
  stdio_write_enabled: false

  http_enabled: false
  http_addr: "127.0.0.1:3001"
  http_path: "/mcp"
  auth_mode: "pre_shared_token"
  require_origin_check: true
  allowed_origins:
    - "https://chatgpt.com"
    - "https://chat.openai.com"
  stateless_http: true
  protocol_versions:
    - "2025-11-25"

  rate_limit:
    read_per_minute: 120
    write_per_minute: 30
    publish_per_10min: 10
    upload_per_10min: 10
```

说明：

- `allowed_origins` 仅为示例，生产环境必须按真实客户端收敛
- `auth_mode=pre_shared_token` 只适用于本地或受控客户端；公网第三方场景需升级为 OAuth 兼容模式
- 若未明确需要 HTTP 接入，建议保持 `http_enabled: false`
- `stdio_write_enabled` 默认为 `false`，避免本地工具被意外赋予写权限
- `stateless_http` 默认为 `true`，除非确有 SDK 兼容要求，否则不启用协议级 session

---

## 十二、运维与密钥管理

### 12.1 Token 签发

`v6` 不要求立即提供图形化 MCP 凭证管理页，首版可通过 CLI 完成：

```text
blogWeb mcp issue-token --name <client-name> --scopes blog.read,blog.publish
blogWeb mcp revoke-token --name <client-name>
```

签发规则：

- token 仅显示一次
- 默认短名称 + 长随机串
- 写权限 token 与只读 token 分离

### 12.2 部署建议

- 公开博客 Web 服务与 MCP HTTP 服务建议分开监听
- MCP HTTP 服务默认不对公网直接暴露
- 若必须公网访问，应至少具备：
  - HTTPS
  - 反向代理 ACL
  - Origin 校验
  - Bearer Token
  - 请求体大小限制
  - 限流

### 12.3 备份与审计

- `mcp_clients` 与 `mcp_audit_logs` 随 SQLite 一并备份
- Redis 中 MCP 限流键、防重放键、临时 handle 不视为核心持久数据，可容忍丢失后重建
- 审计日志应支持按客户端、工具名、时间窗口检索

---

## 十三、验收标准

### 13.1 功能验收

- 本地 AI 客户端可通过 `stdio` 成功发现 `resources`、`tools`、`prompts`
- HTTP MCP 客户端可通过 `Streamable HTTP` 正常调用只读与写工具
- 首版 HTTP 传输在未启用 SSE 时，可仅支持 POST + JSON 响应，并对 GET 返回 `405`
- 只读能力无法读取草稿或未来发布时间文章
- 写工具可创建草稿、更新文章、发布文章、上传图片，且结果与后台 UI 一致

### 13.2 安全验收

- 缺少 token、token 无效、scope 不足时，调用被拒绝
- 非法 `Origin` 被拒绝
- `upload_image` 继续拒绝 SVG、超大文件、伪装 MIME
- `preview_markdown` 和文章输出继续阻断 XSS payload
- MCP 写能力不会绕过 `slug_history`、发布规则和上传安全链路
- 审计表中不出现明文 token、密码、完整图片 base64

### 13.3 兼容性验收

- `v5` 既有公开页面和后台 API 行为不回归
- 不引入新的技术栈冲突
- Web 服务不可用时，不影响本地 `stdio` 模式的协议实现调试

---

## 十四、实施顺序建议

1. 先落数据结构与配置：`mcp_clients`、`mcp_audit_logs`、`mcp` 配置段。
2. 再实现 `stdio` 只读能力：`resources` + `list_articles/get_article/list_categories`。
3. 再补 `preview_markdown`、`create_article_draft`、`update_article`。
4. 然后补 `publish_article`、`upload_image` 和审计/限流。
5. 最后实现 `Streamable HTTP`、Origin 校验、token 鉴权，以及可选的兼容型会话支持。

---

## 十五、最终结论

`v6` 的核心决策是：**当前博客项目应作为 MCP Server 接入，而不是在首阶段引入外部 MCP Client 依赖。**

这样做的收益是：

- 不破坏 `v5` 已完成的安全基线
- 不引入新的服务端技术栈冲突
- 可以把文章查询、草稿写作、发布、上传等博客能力安全开放给 AI 客户端
- 所有高风险操作仍然处于可授权、可审计、可限流、可回收的边界内

因此，`v6` 可以作为当前项目“增加 MCP”时的正式设计基线继续推进实现。
