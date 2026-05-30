# 个人博客系统设计规范（第三版）

> 修订于 2026-05-13，基于第二版设计缺陷复审结果重设计。

## 一、概述

本系统为个人文章发布博客，作者通过 Markdown 编写和发布文章，外部用户可浏览文章、匿名点赞。

### 技术栈

| 层级 | 选型 | 理由 |
|------|------|------|
| 后端语言 | Go 1.22+ | 高性能、单二进制部署 |
| Web 框架 | `gin` | 路由强大、生态丰富 |
| 数据库 | SQLite + GORM | 零配置、单文件、自动迁移 |
| 缓存 / 会话存储 | Redis | 独立部署，高性能会话管理 |
| 会话管理 | `gin-contrib/sessions` + `redis` store | 服务端存储，重启不丢失 |
| CSRF 防护 | CSRF Token（`X-CSRF-Token` 请求头） | 标准防护方案 |
| 密码哈希 | `golang.org/x/crypto/bcrypt` | 官方扩展库 |
| Markdown | `goldmark` + `goldmark-highlighting` | 服务端渲染，含代码高亮 |
| 图片上传 | Gin 原生 `FormFile` + MIME 魔数校验 | 无需额外依赖 |
| 模板引擎 | `html/template` | Go 标准库 |
| 管理后台前端 | React + Semi Design | 完整组件库，交互统一 |
| 管理后台构建 | Vite | 现代前端构建工具 |
| 公开页面样式 | Semi Design CSS 变量 | 与后台视觉统一 |

---

## 二、数据库设计

### `users` — 用户表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | 自增主键 |
| username | TEXT | UNIQUE NOT NULL | 用户名 |
| password | TEXT | NOT NULL | bcrypt 哈希密码 |
| role | TEXT | NOT NULL DEFAULT 'user' | 'admin' / 'user' |
| created_at | DATETIME | NOT NULL | 创建时间 |

### `categories` — 分类表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | 自增主键 |
| name | TEXT | UNIQUE NOT NULL | 分类名称 |
| slug | TEXT | UNIQUE NOT NULL | URL 友好标识 |
| sort_order | INTEGER | NOT NULL DEFAULT 0 | 排序权重 |
| created_at | DATETIME | NOT NULL | 创建时间 |

### `articles` — 文章表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | 自增主键 |
| title | TEXT | NOT NULL | 文章标题 |
| slug | TEXT | UNIQUE NOT NULL | URL 友好标识，可随标题更新 |
| content | TEXT | NOT NULL | Markdown 原始内容 |
| cover_image | TEXT | DEFAULT '' | 封面图路径 |
| excerpt | TEXT | DEFAULT '' | 自动截取正文纯文本前 200 字符 |
| category_id | INTEGER | FK → categories.id, SET NULL | 关联分类 |
| author_id | INTEGER | FK → users.id, NOT NULL | 文章作者 |
| status | TEXT | NOT NULL DEFAULT 'draft' | 'draft' / 'published' |
| is_pinned | INTEGER | NOT NULL DEFAULT 0 | 是否置顶 (0/1) |
| published_at | DATETIME | — | 发布时间（可手动修改） |
| created_at | DATETIME | NOT NULL | 创建时间 |
| updated_at | DATETIME | NOT NULL | 更新时间 |

### `likes` — 点赞表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | 自增主键 |
| article_id | INTEGER | FK → articles.id, NOT NULL | 关联文章 |
| anonymous_id | TEXT | NOT NULL | 匿名客户端标识 |
| ip_address | TEXT | NOT NULL | 客户端 IP（辅助记录） |
| user_agent | TEXT | DEFAULT '' | 浏览器标识（辅助记录） |
| created_at | DATETIME | NOT NULL | 点赞时间 |

唯一约束：`(article_id, anonymous_id)`，防止同一客户端重复点赞。

### `slug_history` — Slug 变更记录表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | 自增主键 |
| article_id | INTEGER | FK → articles.id, NOT NULL | 关联文章 |
| old_slug | TEXT | NOT NULL | 历史 slug |
| created_at | DATETIME | NOT NULL | 变更时间 |

- 文章 slug 更新时，旧 slug 写入此表
- 访问旧 slug 时，301 重定向到新 slug

### Redis 存储

| 用途 | Key 格式 | 说明 |
|------|----------|------|
| 后台会话 | `session:{session_id}` | gin-contrib/sessions redis store 自动管理 |
| CSRF Token | `csrf:{session_id}` | 每个会话关联一个 CSRF token |

---

## 三、会话与安全设计

### Redis 配置（外置配置文件 `config.yaml`）

```yaml
redis:
  addr: "127.0.0.1:6379"
  password: ""
  db: 0
  pool_size: 10
```

### Cookie 策略

| 属性 | 值 | 说明 |
|------|------|------|
| HttpOnly | true | 禁止 JavaScript 访问 |
| Secure | true（生产环境） | 仅 HTTPS 传输 |
| SameSite | Strict | 禁止跨站请求携带 |
| Path | / | — |
| Max-Age | 86400（24 小时） | 过期时间 |

### CSRF 防护（双重策略）

1. **CSRF Token**：登录后服务端生成 CSRF token，与 session 关联存储在 Redis 中。管理后台前端每次写请求（POST/PUT/DELETE）在 `X-CSRF-Token` 请求头中携带。服务端中间件校验请求头中的 token 与 Redis 中存储的 token 是否一致。
2. **SameSite=Strict**：作为辅助防线，禁止浏览器在跨站请求中携带 cookie。

### CSRF Token 获取

管理后台前端登录成功后，调用 `GET /api/admin/csrf-token` 获取 token，存储在内存中，后续所有写请求附带此 token。

### 会话生命周期

- 登录成功 → 创建 session，写入 cookie，生成 CSRF token 存入 Redis
- 24 小时后自动过期，需重新登录
- 登出 → 销毁 session，删除 Redis 中 session 和 CSRF token，清除 cookie
- Redis 重启后 session 不丢失（Redis 持久化）
- 进程重启不影响已登录会话

---

## 四、公开文章可见性规则

公开页面（首页、文章详情、分类页面、API）只返回满足以下条件的文章：

- `status = 'published'`
- `published_at IS NOT NULL AND published_at <= datetime('now')`

详情页访问策略：

- 草稿文章或未到发布时间 → 返回 404
- 已发布文章 → 正常展示
- 旧 slug 访问 → 查询 `slug_history`，存在则 301 重定向到新 slug
- slug 不存在且无历史记录 → 返回 404

---

## 五、核心业务规则

### 5.1 文章状态迁移规则

| 场景 | 行为 |
|------|------|
| 新建文章，status 未传 | 默认 `draft` |
| 新建文章，status=`published`，未传 `published_at` | `published_at` 自动设为当前时间 |
| `draft` → `published`，未传 `published_at` | `published_at` 自动设为当前时间 |
| `draft` → `published`，传了 `published_at` | 使用传入的值 |
| `published` → `draft` | `published_at` 保留原值（不自动清空），但文章立即不可公开访问 |
| 更新已发布文章，未传 `published_at` | `published_at` 保持原值不变 |
| 更新已发布文章，传了 `published_at` | 使用传入的新值 |

### 5.2 Slug 规则

- 新建文章时基于标题自动生成 slug（中文转拼音或直接使用标题的 URL 编码）
- 更新标题时 slug 同步更新
- slug 冲突时自动追加数字后缀：`article-slug` → `article-slug-2` → `article-slug-3`
- slug 更新时，旧 slug 写入 `slug_history` 表
- 访问旧 slug 时，301 重定向到当前正确的 URL

### 5.3 分类删除规则

- 分类下存在 **已发布**（`status='published'`）文章时，禁止删除
- 分类下仅存在草稿文章时，允许删除，删除后文章的 `category_id` 置为 NULL

### 5.4 `article_count` 统计口径

- `GET /api/admin/categories` 返回的 `article_count` 统计该分类下 **全部文章**（含草稿和已发布）
- `GET /api/articles`（公开）可按分类筛选，但仅返回已发布且已到发布时间的文章

### 5.5 点赞规则

- 点赞基于匿名客户端标识（`anonymous_id`），该 ID 由前端生成 UUID 存入 `localStorage`
- **精度边界**：明确接受"设备级匿名点赞"，不是"用户级点赞"。清除浏览器缓存、更换设备、隐私模式下点赞状态不保留
- 服务端校验 `X-Anonymous-Id` 请求头，缺少时返回 400
- 点赞操作使用显式 `action` 参数：`like` / `unlike`
- 不允许重复点赞（409）和重复取消（409）

### 5.6 图片上传规则

| 规则 | 说明 |
|------|------|
| MIME 白名单 | image/jpeg / image/png / image/gif / image/webp |
| MIME 校验 | 服务端读取文件魔数校验，不依赖扩展名 |
| 文件大小 | 最大 5MB |
| 存储路径 | `public/uploads/YYYY/MM/{UUID}.{ext}` |
| 文件名 | UUID 重命名，防止冲突 |
| 覆盖策略 | 不允许覆盖已有文件 |

---

## 六、API 接口契约

所有 API 统一前缀 `/api`，JSON 请求/响应。

### 统一错误码

| HTTP 状态码 | 含义 |
|-------------|------|
| 200 | 成功 |
| 201 | 创建成功 |
| 301 | 永久重定向（slug 变更） |
| 400 | 请求参数错误 |
| 401 | 未登录 |
| 404 | 资源不存在 |
| 409 | 业务冲突（重复点赞、分类下有已发布文章等） |
| 413 | 文件过大 |
| 500 | 服务端内部错误 |

### 6.1 公开接口

#### `GET /api/articles` — 文章列表（cursor 分页）

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| cursor | string | 否 | 分页游标，首次请求不传 |
| category | string | 否 | 分类 slug 筛选 |
| limit | int | 否 | 每页条数，默认 12，最大 50 |

**排序规则（稳定排序键）：** `is_pinned DESC, published_at DESC, id DESC`

**响应 200：**

```json
{
  "list": [
    {
      "id": 1,
      "title": "文章标题",
      "slug": "article-slug",
      "cover_image": "/uploads/2026/05/abc.jpg",
      "excerpt": "文章摘要...",
      "category": { "id": 1, "name": "技术", "slug": "tech" },
      "author": { "id": 1, "username": "admin" },
      "is_pinned": true,
      "like_count": 42,
      "published_at": "2026-05-13T10:00:00Z"
    }
  ],
  "next_cursor": "{\"is_pinned\":0,\"published_at\":\"2026-05-10T08:00:00Z\",\"id\":15}",
  "has_more": true
}
```

#### `GET /api/articles/:slug` — 文章详情

**响应 200：**

```json
{
  "id": 1,
  "title": "文章标题",
  "slug": "article-slug",
  "content_html": "<h1>渲染后的 HTML</h1>",
  "cover_image": "/uploads/2026/05/abc.jpg",
  "category": { "id": 1, "name": "技术", "slug": "tech" },
  "author": { "id": 1, "username": "admin" },
  "is_pinned": true,
  "like_count": 42,
  "user_liked": true,
  "published_at": "2026-05-13T10:00:00Z",
  "created_at": "2026-05-10T08:00:00Z",
  "updated_at": "2026-05-12T15:30:00Z"
}
```

- `user_liked` 基于请求中的 `X-Anonymous-Id` 判断
- 未发布文章返回 404：`{ "code": 404, "message": "文章不存在" }`
- 旧 slug 访问 → 301 重定向到新 slug

#### `POST /api/articles/:slug/like` — 点赞/取消点赞

**请求头：** `X-Anonymous-Id: <anonymous_id>`（必填）

**请求体：**

```json
{ "action": "like" }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| action | string | 是 | 'like' / 'unlike' |

**响应 200（点赞成功）：**

```json
{ "liked": true, "like_count": 43 }
```

**响应 200（取消点赞成功）：**

```json
{ "liked": false, "like_count": 42 }
```

**错误响应：**

```json
{ "code": 400, "message": "无效的操作，action 必须为 like 或 unlike" }
```

```json
{ "code": 400, "message": "缺少 X-Anonymous-Id 请求头" }
```

```json
{ "code": 409, "message": "已经点过赞了" }
```

```json
{ "code": 409, "message": "尚未点赞，无法取消" }
```

#### `POST /api/likes/batch` — 批量查询点赞状态

**请求头：** `X-Anonymous-Id: <anonymous_id>`（必填）

**请求体：**

```json
{ "article_slugs": ["slug-1", "slug-2", "slug-3"] }
```

**响应 200：**

```json
{
  "liked_map": {
    "slug-1": true,
    "slug-2": false
  }
}
```

- 不存在的 slug 不会出现在 `liked_map` 中

### 6.2 管理后台接口

所有接口需登录态（session cookie）。写接口还需 CSRF Token。未登录统一返回 `{ "code": 401, "message": "请先登录" }`。

#### `POST /api/admin/login` — 登录

**请求体：**

```json
{ "username": "admin", "password": "your-password" }
```

**响应 200：**

```json
{ "user": { "id": 1, "username": "admin", "role": "admin" } }
```

- 登录成功后设置 session cookie，并生成 CSRF token 存入 Redis

**响应 401：**

```json
{ "code": 401, "message": "用户名或密码错误" }
```

#### `POST /api/admin/logout` — 退出登录

**响应 200：**

```json
{ "message": "已退出登录" }
```

- 销毁 Redis 中的 session 和 CSRF token，清除 cookie

#### `GET /api/admin/csrf-token` — 获取 CSRF Token

需登录态。返回当前 session 关联的 CSRF token。

**响应 200：**

```json
{ "csrf_token": "random-token-string" }
```

#### `GET /api/admin/articles` — 文章列表（页码分页）

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | int | 否 | 页码，默认 1 |
| page_size | int | 否 | 每页条数，默认 20，最大 100 |
| status | string | 否 | 筛选：'draft' / 'published' |
| category_id | int | 否 | 按分类筛选 |
| keyword | string | 否 | 标题模糊搜索 |
| sort_by | string | 否 | 排序字段：'published_at' / 'created_at' / 'updated_at' / 'like_count' |
| sort_order | string | 否 | 'asc' / 'desc'，默认 'desc' |

**响应 200：**

```json
{
  "list": [
    {
      "id": 1,
      "title": "文章标题",
      "slug": "article-slug",
      "status": "published",
      "is_pinned": true,
      "category": { "id": 1, "name": "技术" },
      "like_count": 42,
      "published_at": "2026-05-13T10:00:00Z",
      "created_at": "2026-05-10T08:00:00Z",
      "updated_at": "2026-05-12T15:30:00Z"
    }
  ],
  "page": 1,
  "page_size": 20,
  "total": 100
}
```

#### `GET /api/admin/articles/:id` — 文章详情

**响应 200：**

```json
{
  "id": 1,
  "title": "文章标题",
  "slug": "article-slug",
  "content": "# Markdown 原始内容",
  "cover_image": "/uploads/2026/05/abc.jpg",
  "category_id": 1,
  "status": "published",
  "is_pinned": false,
  "published_at": "2026-05-13T10:00:00Z",
  "created_at": "2026-05-10T08:00:00Z",
  "updated_at": "2026-05-12T15:30:00Z"
}
```

- 返回 `content` 原始 Markdown（用于编辑器），公开接口返回 `content_html`

#### `POST /api/admin/articles` — 新建文章

需 CSRF Token。

**请求体：**

```json
{
  "title": "文章标题",
  "content": "# Markdown 内容",
  "cover_image": "/uploads/2026/05/abc.jpg",
  "category_id": 1,
  "status": "draft",
  "is_pinned": false,
  "published_at": "2026-05-13T10:00:00Z"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| title | string | 是 | 文章标题 |
| content | string | 是 | Markdown 原始内容 |
| cover_image | string | 否 | 封面图路径 |
| category_id | int | 否 | 分类 ID |
| status | string | 否 | 默认 'draft' |
| is_pinned | bool | 否 | 默认 false |
| published_at | string | 否 | RFC3339 格式 |

**响应 201：**

```json
{ "id": 2, "slug": "article-slug" }
```

#### `PUT /api/admin/articles/:id` — 更新文章

需 CSRF Token。

**请求体：** 所有字段可选，传入即更新。字段含义同新建。

**响应 200：**

```json
{ "id": 1, "slug": "new-slug" }
```

- 如果 title 变更导致 slug 变更，响应中返回新 slug，旧 slug 自动写入 `slug_history`

#### `DELETE /api/admin/articles/:id` — 删除文章

需 CSRF Token。同时清理 `slug_history` 中该文章的所有记录。

**响应 200：**

```json
{ "message": "删除成功" }
```

#### `GET /api/admin/categories` — 分类列表

**响应 200：**

```json
{
  "list": [
    { "id": 1, "name": "技术", "slug": "tech", "sort_order": 0, "article_count": 5 },
    { "id": 2, "name": "生活", "slug": "life", "sort_order": 1, "article_count": 3 }
  ]
}
```

- `article_count` 统计该分类下 **全部文章**（含草稿和已发布）

#### `POST /api/admin/categories` — 新建分类

需 CSRF Token。

**请求体：**

```json
{ "name": "新分类", "slug": "new-category" }
```

**响应 201：**

```json
{ "id": 3, "name": "新分类", "slug": "new-category" }
```

#### `PUT /api/admin/categories/:id` — 更新分类

需 CSRF Token。

**请求体：**

```json
{ "name": "新名称", "slug": "new-slug" }
```

**响应 200：**

```json
{ "id": 1, "name": "新名称", "slug": "new-slug" }
```

#### `DELETE /api/admin/categories/:id` — 删除分类

需 CSRF Token。

- 分类下存在 **已发布**（`status='published'`）文章时，禁止删除
- 仅存在草稿文章时，允许删除，文章 `category_id` 置为 NULL

**响应 200：**

```json
{ "message": "删除成功" }
```

**响应 409：**

```json
{ "code": 409, "message": "该分类下存在 5 篇已发布文章，无法删除" }
```

#### `PUT /api/admin/categories/sort` — 批量排序分类

需 CSRF Token。

**请求体：**

```json
{ "ids": [3, 1, 2] }
```

**响应 200：**

```json
{ "message": "排序更新成功" }
```

#### `POST /api/admin/upload` — 图片上传

需登录态 + CSRF Token。请求格式 `multipart/form-data`，字段名 `file`。

**响应 200：**

```json
{ "url": "/uploads/2026/05/a1b2c3d4.jpg", "filename": "a1b2c3d4.jpg" }
```

**错误响应：**

```json
{ "code": 400, "message": "不支持的文件类型，仅允许 jpg/png/gif/webp" }
```

```json
{ "code": 413, "message": "文件大小超过 5MB 限制" }
```

### 6.3 预留接口（后端逻辑就位，前端暂不开放）

| 路由 | 说明 |
|------|------|
| `POST /api/register` | 用户注册 |
| `POST /api/login` | 用户登录 |
| `POST /api/logout` | 用户退出 |

---

## 七、路由总览

### 公开页面路由

| 路由 | 处理方式 | 说明 |
|------|----------|------|
| `GET /` | Go 模板 | 首页 |
| `GET /articles/:slug` | Go 模板 | 文章详情，旧 slug 301 重定向 |
| `GET /categories/:slug` | Go 模板 | 分类文章列表 |
| `GET /api/articles` | JSON | 文章列表（cursor 分页） |
| `GET /api/articles/:slug` | JSON | 文章详情（含 content_html） |
| `POST /api/articles/:slug/like` | JSON | 点赞/取消点赞 |
| `POST /api/likes/batch` | JSON | 批量查询点赞状态 |

### 管理后台路由

| 路由 | 处理方式 | 说明 |
|------|----------|------|
| `POST /api/admin/login` | JSON | 登录 |
| `POST /api/admin/logout` | JSON | 退出登录 |
| `GET /api/admin/csrf-token` | JSON | 获取 CSRF Token |
| `GET /api/admin/articles` | JSON | 文章列表（页码分页） |
| `GET /api/admin/articles/:id` | JSON | 文章详情（含原始 Markdown） |
| `POST /api/admin/articles` | JSON | 新建文章（需 CSRF Token） |
| `PUT /api/admin/articles/:id` | JSON | 更新文章（需 CSRF Token） |
| `DELETE /api/admin/articles/:id` | JSON | 删除文章（需 CSRF Token） |
| `GET /api/admin/categories` | JSON | 分类列表 |
| `POST /api/admin/categories` | JSON | 新建分类（需 CSRF Token） |
| `PUT /api/admin/categories/:id` | JSON | 更新分类（需 CSRF Token） |
| `DELETE /api/admin/categories/:id` | JSON | 删除分类（需 CSRF Token） |
| `PUT /api/admin/categories/sort` | JSON | 批量排序分类（需 CSRF Token） |
| `POST /api/admin/upload` | JSON | 图片上传（需 CSRF Token） |
| `GET /admin/*` | 静态文件 | React SPA 入口 |

---

## 八、前端设计

### 8.1 公开页面（Go 模板 + 原生 JS）

- **首页**：时间倒序卡片网格布局，置顶文章优先显示并带有"置顶"标识。无限滚动通过 IntersectionObserver 监听哨兵元素，fetch `/api/articles` 追加渲染。每张卡片展示封面图、标题、摘要、发布时间、分类标签、点赞数和点赞按钮（空心/实心）。首次加载时调用 `/api/likes/batch` 批量获取点赞状态。
- **文章详情页**：goldmark 服务端渲染 Markdown（含代码高亮），底部点赞按钮 + 点赞数。
- **分类页面**：与首页类似，按分类筛选。
- **301 重定向**：访问旧 slug 时，服务端返回 301，浏览器自动跳转到新 URL。
- **响应式**：桌面 3 列 → 平板 2 列 → 手机 1 列。
- **样式**：引入 Semi Design CSS 变量，自定义样式遵循 Design Tokens 规范。
- **匿名标识**：页面首次加载时生成 UUID 存入 `localStorage`，所有点赞请求通过 `X-Anonymous-Id` 请求头携带。明确接受"设备级匿名"，清缓存/换设备/隐私模式下点赞状态不恢复。

### 8.2 管理后台（React SPA + Semi Design）

- **登录页**：Semi Design Form 组件，账号密码登录。登录成功后立即调用 `GET /api/admin/csrf-token` 获取 CSRF token 并存储在内存中。
- **仪表盘**：Semi Table 展示文章列表（标题、分类、状态、点赞数、发布时间），支持分页、状态筛选、分类筛选、关键词搜索、多字段排序。提供新建、编辑、删除操作入口。
- **文章编辑**：EasyMDE 编辑器集成到 React，支持封面图上传、分类选择、草稿/发布切换、置顶开关、修改发布时间。编辑页通过 `GET /api/admin/articles/:id` 获取原始 Markdown 初始化编辑器。发布/状态切换遵循状态迁移规则。
- **分类管理**：Semi Table + Modal 展示分类列表（含 `article_count`），支持拖拽排序，排序后调用 `PUT /api/admin/categories/sort` 提交。删除分类时，若有已发布文章则提示错误。

---

## 九、项目结构

```
blogWeb/
├── main.go
├── go.mod
├── config.yaml                  # 外置配置文件（Redis、数据库等）
├── data/                        # SQLite 数据库文件
├── config/
│   └── config.go                # 配置加载（解析 config.yaml）
├── internal/
│   ├── middleware/
│   │   ├── auth.go              # 登录验证中间件
│   │   └── csrf.go              # CSRF Token 校验中间件
│   ├── handler/
│   │   ├── article.go
│   │   ├── category.go
│   │   ├── auth.go
│   │   ├── upload.go
│   │   └── like.go
│   ├── model/
│   │   ├── user.go
│   │   ├── article.go
│   │   ├── category.go
│   │   ├── like.go
│   │   └── slug_history.go
│   └── service/
│       ├── article.go
│       ├── category.go
│       ├── auth.go
│       └── like.go
├── templates/
│   ├── layouts/
│   │   └── base.html
│   └── public/
│       ├── index.html
│       ├── article.html
│       └── category.html
├── client/                      # React 管理后台
│   ├── package.json
│   ├── vite.config.js
│   └── src/
│       ├── main.jsx
│       ├── App.jsx
│       ├── pages/
│       │   ├── Login.jsx
│       │   ├── Dashboard.jsx
│       │   ├── ArticleEdit.jsx
│       │   └── Categories.jsx
│       ├── components/
│       └── utils/
│           └── api.js           # API 请求封装（含 CSRF token 管理）
├── public/
│   ├── css/
│   ├── js/
│   ├── admin/                   # React 构建产物
│   └── uploads/                 # 上传图片（按年月分目录）
└── migrations/                  # 数据库迁移脚本（版本化管理）
    └── 001_init.sql
```

---

## 十、部署与运维

### 外置配置文件 (`config.yaml`)

```yaml
server:
  port: 3000

database:
  path: "data/blog.db"

redis:
  addr: "127.0.0.1:6379"
  password: ""
  db: 0
  pool_size: 10

session:
  secret: "<随机 32 位字符串>"
  max_age: 86400  # 24 小时，单位秒

upload:
  dir: "public/uploads"
  max_size: 5242880  # 5MB
  allowed_types:
    - image/jpeg
    - image/png
    - image/gif
    - image/webp

admin:
  init_username: "admin"
  init_password: "<初始密码>"
```

### 开发环境

- 确保 Redis 已启动
- 后端：`go run main.go`
- 前端：`cd client && npm run dev`（Vite 代理 API 到后端）

### 生产构建

- 前端：`cd client && npm run build`（输出到 `public/admin/`）
- 后端：`go build -o blogWeb .`
- 运行：`./blogWeb`
- 使用 `pm2` 或 `systemd` 管理进程（单实例部署）

### 初始化流程

1. 首次启动运行数据库迁移脚本
2. 检查是否存在 admin 用户，不存在则根据 `config.yaml` 创建
3. 连接 Redis，连接失败则拒绝启动
4. 种子数据（可选）：示例分类

### 备份策略

- SQLite：使用 `.backup` 命令热备份
- Redis：依赖 Redis 自身持久化（RDB / AOF）
- 上传文件：定期备份 `public/uploads/` 目录