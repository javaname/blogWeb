# 个人博客系统设计规范（第五版）

> 修订于 2026-05-13，基于第四版安全复审结果增强。

## 一、概述

本系统为个人文章发布博客，作者通过 Markdown 编写和发布文章，外部用户可浏览文章、匿名点赞。

### 技术栈

| 层级 | 选型 | 理由 |
|------|------|------|
| 后端语言 | Go 1.22+ | 高性能、单二进制部署 |
| Web 框架 | `gin` | 路由强大、生态丰富 |
| 数据库 | SQLite + GORM + 版本化迁移脚本 | 零配置、单文件、结构变更可追踪 |
| 缓存 / 会话存储 | Redis | 服务端会话、CSRF token、限流计数存储 |
| 会话管理 | `gin-contrib/sessions` + `redis` store | 服务端存储，支持进程重启后继续登录 |
| CSRF 防护 | CSRF Token（`X-CSRF-Token` 请求头） | 标准防护方案 |
| 密码哈希 | `golang.org/x/crypto/bcrypt` | 官方扩展库 |
| Markdown | `goldmark` + `goldmark-highlighting` + HTML Sanitizer | 服务端渲染并做 XSS 清洗 |
| 图片上传 | Gin 原生 `FormFile` + MIME 魔数校验 + 重编码 | 无需额外依赖，增强上传安全 |
| 模板引擎 | `html/template` | Go 标准库，自带转义 |
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
| slug | TEXT | UNIQUE NOT NULL | 当前 slug，全局唯一，不得与历史 slug 冲突 |
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

### `slug_history` — Slug 历史保留表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | 自增主键 |
| article_id | INTEGER | FK → articles.id, NULL | 当前指向文章；文章删除后置 NULL |
| old_slug | TEXT | UNIQUE NOT NULL | 历史 slug，全局唯一且不可复用 |
| created_at | DATETIME | NOT NULL | 写入时间 |

- 文章 slug 更新时，旧 slug 写入此表
- 当前 slug 与历史 slug 共同组成“全局保留池”，一旦使用过就不可被其他文章复用
- 访问路径解析顺序：先查 `articles.slug`，未命中再查 `slug_history.old_slug`
- 迁移脚本要求：
  - `FOREIGN KEY(article_id) REFERENCES articles(id) ON DELETE SET NULL`
  - `UNIQUE INDEX uq_slug_history_old_slug(old_slug)`
  - `INDEX idx_slug_history_article_id(article_id)`
- 文章删除时：
  - 若当前 slug 尚未写入 `slug_history`，先补写为保留记录
  - 该文章关联的所有 `slug_history.article_id` 置为 `NULL`
  - 所有已使用过的 slug 继续保留，但访问时返回 404

### Redis 存储

| 用途 | Key 格式 | 说明 |
|------|----------|------|
| 后台会话 | `session:{session_id}` | `gin-contrib/sessions` redis store 自动管理 |
| CSRF Token | `csrf:{session_id}` | 每个会话关联一个 CSRF token |
| 登录限流 | `login_rate:{ip}` / `login_fail:{username}` | 登录速率与失败次数控制 |
| 点赞限流 | `like_rate:{ip}` / `like_article_rate:{ip}:{article_id}` | 点赞接口防刷 |

---

## 三、会话、安全与匿名身份

### Redis 配置（外置配置文件 `config.yaml`）

```yaml
redis:
  addr: "127.0.0.1:6379"
  password: ""
  db: 0
  pool_size: 10
```

### 后台 Session Cookie 策略

| 属性 | 值 | 说明 |
|------|------|------|
| HttpOnly | true | 禁止 JavaScript 访问 |
| Secure | true（生产环境） | 仅 HTTPS 传输 |
| SameSite | Strict | 禁止跨站请求携带 |
| Path | / | — |
| Max-Age | 86400（24 小时） | 绝对过期时间 |

### Session 安全加固

- 登录成功后必须重新生成 session id，防止 session fixation
- 后台空闲超时：连续 2 小时无操作失效，需重新登录
- 绝对过期时间：24 小时
- 登出时销毁 session、CSRF token，并清除 cookie

### 公开站匿名身份策略

公开站使用 `anonymous_id` 作为设备级匿名身份。正常情况下以 cookie 为真源、`localStorage` 为前端副本；若浏览器禁用 cookie，则退化为仅前端可见的 `localStorage` 身份。

#### `anonymous_id` Cookie 策略

| 属性 | 值 | 说明 |
|------|------|------|
| 名称 | `anonymous_id` | 匿名身份标识 |
| HttpOnly | false | 前端需读取并同步到 `localStorage` |
| Secure | true（生产环境） | 仅 HTTPS 传输 |
| SameSite | Lax | 减少跨站携带风险，保留站内可用性 |
| Path | / | — |
| Max-Age | 31536000（1 年） | 长期保持匿名身份稳定 |

#### 匿名身份生成与同步规则

1. 首次访问公开页面时，如请求未携带 `anonymous_id` cookie，服务端生成 UUID 并在响应中写入 cookie。
2. Go 模板页面渲染时：
   - 若请求已携带 `anonymous_id` cookie，则服务端基于该 ID 计算 `user_liked` 等首屏状态。
   - 若当前请求没有该 cookie，则首屏按“未点赞”渲染，同时在响应中下发新 cookie。
3. 前端 JS 启动后，若 cookie 可用，则以 cookie 作为匿名身份真源：
   - `localStorage` 不存在时，从 cookie 同步写入
   - `localStorage` 与 cookie 不一致时，以 cookie 覆盖 `localStorage`
4. 若检测到浏览器禁用 cookie：
   - 若 `localStorage` 中已有 `anonymous_id`，继续复用
   - 若 `localStorage` 中不存在，则前端生成临时 UUID 写入 `localStorage`
   - 此模式下服务端模板首屏无法识别点赞态，统一按“未点赞”渲染；页面 hydration 后由前端发起状态请求校正
5. 所有点赞相关 AJAX 请求通过 `X-Anonymous-Id` 请求头携带该值；若 cookie 与 `localStorage` 同时可用，则请求头值与 cookie 保持一致。

### CSRF 防护

1. 登录后服务端生成 CSRF token，与 session 关联存储在 Redis 中。
2. 管理后台前端通过 `GET /api/admin/csrf-token` 获取 token，并仅存放在内存中。
3. 所有后台写请求（POST/PUT/DELETE）都必须携带 `X-CSRF-Token`。
4. `SameSite=Strict` 作为辅助防线，减少跨站携带后台 cookie 的可能。

### 后台授权边界

- 所有 `/api/admin/*` 接口都要求：
  - 已登录 session
  - `users.role = 'admin'`
- 非 admin 用户访问后台接口统一返回：

```json
{ "code": 403, "message": "无权限访问" }
```

---

## 四、内容安全、上传安全与公开可见性

### Markdown 渲染安全策略

- Markdown 内容使用 `goldmark` 渲染后，必须经过 HTML Sanitizer 白名单清洗
- 默认不信任原始 HTML；即使开启部分 HTML 扩展，也必须经过清洗后输出
- 明确禁止：
  - `<script>`
  - `<iframe>`
  - 内联事件属性，如 `onclick`
  - `style` 中的危险内容
  - `javascript:` / `data:text/html` 等危险协议
- 链接与图片 URL 仅允许：
  - `http`
  - `https`
  - 站内相对路径
- 输出到模板或 JSON 的 `content_html` 必须为 sanitization 后结果

### 上传安全策略

- 仅允许位图格式：
  - `image/jpeg`
  - `image/png`
  - `image/gif`
  - `image/webp`
- 明确禁止 `svg`
- 服务端读取文件魔数判断真实类型，不依赖扩展名
- 上传成功后按检测结果重新确定扩展名，不沿用原始文件名扩展名
- 可选但推荐：对图片做重新编码后再落盘，降低 polyglot 文件风险
- 上传目录只做静态文件托管，禁止脚本执行
- 静态资源服务必须返回：
  - 正确的 `Content-Type`
  - `X-Content-Type-Options: nosniff`

### 公开文章可见性

公开页面（首页、文章详情、分类页面、公开 API）只返回满足以下条件的文章：

- `status = 'published'`
- `published_at IS NOT NULL AND published_at <= datetime('now')`

### 路径解析优先级

访问 `/articles/:slug` 或 `GET /api/articles/:slug` 时，按以下顺序解析：

1. 先按 `articles.slug` 查当前文章
2. 未命中时，再按 `slug_history.old_slug` 查历史 slug
3. 若命中历史 slug：
   - 当目标文章当前满足公开可见性时，返回 `301` 重定向到新 slug
   - 当目标文章不存在、已删除、为草稿或未到发布时间时，直接返回 `404`
4. 都未命中时，返回 `404`

### 详情页访问策略

- 草稿文章或未到发布时间 → 返回 404
- 已发布文章 → 正常展示
- 历史 slug 且目标文章当前可见 → 301 重定向到新 slug
- 历史 slug 但目标文章当前不可见 / 已删除 → 返回 404
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

- 新建文章时基于标题自动生成 slug（中文转拼音或直接使用标题的 URL 安全编码）
- 更新标题时 slug 同步更新
- slug 生成时必须同时避开：
  - `articles.slug`
  - `slug_history.old_slug`
- slug 冲突时自动追加数字后缀：`article-slug` → `article-slug-2` → `article-slug-3`
- slug 一旦进入当前表或历史表，即永久保留，不允许被其他文章复用
- 路由解析优先匹配当前 slug，其次匹配历史 slug

### 5.3 分类删除规则

- 分类下存在 **已发布**（`status='published'`）文章时，禁止删除
- 分类下仅存在草稿文章时，允许删除，删除后文章的 `category_id` 置为 `NULL`

### 5.4 `article_count` 统计口径

- `GET /api/admin/categories` 返回的 `article_count` 统计该分类下 **全部文章**（含草稿和已发布）
- 公开列表按分类筛选时，仅返回已发布且已到发布时间的文章

### 5.5 点赞规则

- 点赞基于匿名客户端标识（`anonymous_id`）
- 精度边界：明确接受“设备级匿名点赞”，不是“用户级点赞”
- 清除浏览器缓存、更换设备、隐私模式下点赞状态不保留
- 服务端模板首屏读取 `anonymous_id` cookie 渲染详情页点赞按钮初始状态
- 若浏览器禁用 cookie，则公开页面仍可浏览，但点赞能力退化为前端基于 `localStorage` 的弱一致模式
- 首页列表页首屏统一渲染为空心点赞按钮；加载完成后调用 `/api/likes/batch` 批量补齐当前页卡片点赞状态
- 点赞操作使用显式 `action` 参数：`like` / `unlike`
- 不允许重复点赞（409）和重复取消（409）

### 5.6 登录防暴力破解规则

- `POST /api/admin/login` 必须做限流与失败控制
- 推荐默认策略：
  - 单 IP：10 分钟内最多 20 次登录尝试
  - 单用户名：连续失败 5 次后冷却 15 分钟
  - 成功登录后清空该用户名的失败计数
- 所有登录失败与冷却命中都记录安全日志

### 5.7 点赞反刷规则

- `POST /api/articles/:slug/like` 与 `POST /api/likes/batch` 需做基础限流
- 推荐默认策略：
  - 单 IP：1 分钟内最多 60 次点赞相关请求
  - 单文章单 IP：10 分钟内最多 20 次点赞状态变更尝试
- 命中限流时返回 429
- 后台可查看异常点赞频率日志或统计

### 5.8 图片上传规则

| 规则 | 说明 |
|------|------|
| MIME 白名单 | image/jpeg / image/png / image/gif / image/webp |
| MIME 校验 | 服务端读取文件魔数校验，不依赖扩展名 |
| 文件大小 | 最大 5MB |
| 存储路径 | `public/uploads/YYYY/MM/{UUID}.{ext}` |
| 文件名 | UUID 重命名，防止冲突 |
| 覆盖策略 | 不允许覆盖已有文件 |
| 类型限制 | 明确禁止 SVG |
| 安全策略 | 推荐重编码，禁止脚本执行，返回 `nosniff` |

---

## 六、API 接口契约

所有 API 统一前缀 `/api`，JSON 请求/响应。

### 统一错误码

| HTTP 状态码 | 含义 |
|-------------|------|
| 200 | 成功 |
| 201 | 创建成功 |
| 301 | 永久重定向（历史 slug 命中且目标文章当前可见） |
| 400 | 请求参数错误 |
| 401 | 未登录 |
| 403 | 已登录但无权限 |
| 404 | 资源不存在或当前不可公开访问 |
| 409 | 业务冲突（重复点赞、分类下有已发布文章等） |
| 413 | 文件过大 |
| 429 | 请求过于频繁 / 命中限流 |
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

- `content_html` 必须为 sanitization 后结果
- `user_liked` 判断优先使用 `X-Anonymous-Id`，若请求头缺失则回退使用 `anonymous_id` cookie；两者都不存在时返回 `false`
- 未发布文章返回 404：`{ "code": 404, "message": "文章不存在" }`
- 历史 slug 且目标文章当前可见 → 301 重定向到新 slug
- 历史 slug 但目标文章当前不可见 / 已删除 → 404

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

```json
{ "code": 429, "message": "请求过于频繁，请稍后再试" }
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
- 需参与点赞限流体系

### 6.2 管理后台接口

所有 `/api/admin/*` 接口都需 `admin` 登录态（session cookie）。写接口还需 CSRF Token。未登录统一返回 `{ "code": 401, "message": "请先登录" }`；非 admin 返回 `{ "code": 403, "message": "无权限访问" }`。

#### `POST /api/admin/login` — 登录

**请求体：**

```json
{ "username": "admin", "password": "your-password" }
```

**响应 200：**

```json
{ "user": { "id": 1, "username": "admin", "role": "admin" } }
```

- 登录成功后：
  - 重新生成 session id
  - 设置 session cookie
  - 生成 CSRF token 存入 Redis
  - 清空该用户名的失败计数

**响应 401：**

```json
{ "code": 401, "message": "用户名或密码错误" }
```

**响应 429：**

```json
{ "code": 429, "message": "登录尝试过于频繁，请稍后再试" }
```

#### `POST /api/admin/logout` — 退出登录

**响应 200：**

```json
{ "message": "已退出登录" }
```

- 销毁 Redis 中的 session 和 CSRF token，清除 cookie

#### `GET /api/admin/csrf-token` — 获取 CSRF Token

需 admin 登录态。返回当前 session 关联的 CSRF token。

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

需 admin 登录态 + CSRF Token。

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

需 admin 登录态 + CSRF Token。

**请求体：** 所有字段可选，传入即更新。字段含义同新建。

**响应 200：**

```json
{ "id": 1, "slug": "new-slug" }
```

- 如果 title 变更导致 slug 变更：
  - 旧 slug 自动写入 `slug_history`
  - 新 slug 必须同时避开当前 slug 和全部历史 slug

#### `DELETE /api/admin/articles/:id` — 删除文章

需 admin 登录态 + CSRF Token。

**删除行为：**

- 若当前 slug 尚未进入 `slug_history`，先写入保留记录
- 将该文章关联的全部 `slug_history.article_id` 置为 `NULL`
- 删除文章主记录

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

需 admin 登录态 + CSRF Token。

**请求体：**

```json
{ "name": "新分类", "slug": "new-category" }
```

**响应 201：**

```json
{ "id": 3, "name": "新分类", "slug": "new-category" }
```

#### `PUT /api/admin/categories/:id` — 更新分类

需 admin 登录态 + CSRF Token。

**请求体：**

```json
{ "name": "新名称", "slug": "new-slug" }
```

**响应 200：**

```json
{ "id": 1, "name": "新名称", "slug": "new-slug" }
```

#### `DELETE /api/admin/categories/:id` — 删除分类

需 admin 登录态 + CSRF Token。

- 分类下存在 **已发布**（`status='published'`）文章时，禁止删除
- 仅存在草稿文章时，允许删除，文章 `category_id` 置为 `NULL`

**响应 200：**

```json
{ "message": "删除成功" }
```

**响应 409：**

```json
{ "code": 409, "message": "该分类下存在 5 篇已发布文章，无法删除" }
```

#### `PUT /api/admin/categories/sort` — 批量排序分类

需 admin 登录态 + CSRF Token。

**请求体：**

```json
{ "ids": [3, 1, 2] }
```

**响应 200：**

```json
{ "message": "排序更新成功" }
```

#### `POST /api/admin/upload` — 图片上传

需 admin 登录态 + CSRF Token。请求格式 `multipart/form-data`，字段名 `file`。

**服务端额外要求：**

- 明确禁止 `svg`
- 真实类型判定后重建扩展名
- 推荐重编码后落盘

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

### 6.3 未来规划（当前版本不实现）

| 路由 | 说明 |
|------|------|
| `POST /api/register` | 普通用户注册 |
| `POST /api/login` | 普通用户登录 |
| `POST /api/logout` | 普通用户退出 |

---

## 七、路由总览

### 公开页面路由

| 路由 | 处理方式 | 说明 |
|------|----------|------|
| `GET /` | Go 模板 | 首页 |
| `GET /articles/:slug` | Go 模板 | 文章详情；先查当前 slug，再查历史 slug；历史 slug 仅在目标文章当前可见时返回 301 |
| `GET /categories/:slug` | Go 模板 | 分类文章列表 |
| `GET /api/articles` | JSON | 文章列表（cursor 分页） |
| `GET /api/articles/:slug` | JSON | 文章详情（含 `content_html` / `user_liked`） |
| `POST /api/articles/:slug/like` | JSON | 点赞/取消点赞（含限流） |
| `POST /api/likes/batch` | JSON | 批量查询点赞状态（含限流） |

### 管理后台路由

| 路由 | 处理方式 | 说明 |
|------|----------|------|
| `POST /api/admin/login` | JSON | 登录（含防暴力破解） |
| `POST /api/admin/logout` | JSON | 退出登录 |
| `GET /api/admin/csrf-token` | JSON | 获取 CSRF Token |
| `GET /api/admin/articles` | JSON | 文章列表（需 admin） |
| `GET /api/admin/articles/:id` | JSON | 文章详情（需 admin） |
| `POST /api/admin/articles` | JSON | 新建文章（需 admin + CSRF） |
| `PUT /api/admin/articles/:id` | JSON | 更新文章（需 admin + CSRF） |
| `DELETE /api/admin/articles/:id` | JSON | 删除文章（需 admin + CSRF） |
| `GET /api/admin/categories` | JSON | 分类列表（需 admin） |
| `POST /api/admin/categories` | JSON | 新建分类（需 admin + CSRF） |
| `PUT /api/admin/categories/:id` | JSON | 更新分类（需 admin + CSRF） |
| `DELETE /api/admin/categories/:id` | JSON | 删除分类（需 admin + CSRF） |
| `PUT /api/admin/categories/sort` | JSON | 批量排序分类（需 admin + CSRF） |
| `POST /api/admin/upload` | JSON | 图片上传（需 admin + CSRF） |
| `GET /admin/*` | 静态文件 | React SPA 入口 |

---

## 八、前端设计

### 8.1 公开页面（Go 模板 + 原生 JS）

- **首页**：时间倒序卡片网格布局，置顶文章优先显示并带有“置顶”标识。无限滚动通过 `IntersectionObserver` 监听哨兵元素，fetch `/api/articles` 追加渲染。每张卡片展示封面图、标题、摘要、发布时间、分类标签、点赞数和点赞按钮。
- **首页点赞状态**：首屏 HTML 不要求逐卡片服务端渲染点赞态；默认统一渲染为空心点赞按钮，页面加载后调用 `/api/likes/batch` 批量补齐当前页点赞状态。
- **文章详情页**：Go 模板在服务端渲染 Markdown 与点赞按钮。若请求携带 `anonymous_id` cookie，则服务端直接渲染初始 `user_liked` 状态；若没有 cookie，则默认未点赞。若浏览器禁用 cookie，但 `localStorage` 中存在 `anonymous_id`，则页面 hydration 后由前端补发详情状态请求校正按钮状态。
- **分类页面**：与首页类似，按分类筛选。
- **历史 slug 跳转**：访问旧 slug 且目标文章当前可见时，服务端返回 301；目标文章不可见或已删除时返回 404。
- **匿名标识同步**：前端启动后优先将 `anonymous_id` cookie 同步到 `localStorage`；若 cookie 不可用，则退化使用 `localStorage` 中的匿名 ID。所有点赞请求通过 `X-Anonymous-Id` 请求头发送。
- **安全要求**：前端不得直接信任 Markdown 原文，公开渲染只消费服务端返回的已清洗 `content_html`。

### 8.2 管理后台（React SPA + Semi Design）

- **登录页**：账号密码登录。登录成功后立即调用 `GET /api/admin/csrf-token` 获取 CSRF token，并仅保存在内存中。登录页对连续失败提示冷却信息。
- **仪表盘**：文章列表支持分页、状态筛选、分类筛选、关键词搜索、多字段排序。
- **文章编辑**：支持封面图上传、分类选择、草稿/发布切换、置顶开关、修改发布时间。若标题变化导致 slug 更新，前端以接口返回的新 slug 为准。
- **分类管理**：展示 `article_count`，支持拖拽排序。删除分类时，若有已发布文章则提示错误；若仅有草稿文章，则允许删除并将这些文章分类置空。

---

## 九、项目结构

```text
blogWeb/
├── main.go
├── go.mod
├── config.yaml
├── data/
├── config/
│   └── config.go
├── internal/
│   ├── middleware/
│   │   ├── auth.go
│   │   ├── csrf.go
│   │   ├── admin_role.go
│   │   ├── rate_limit.go
│   │   └── security_headers.go
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
│       ├── like.go
│       ├── sanitizer.go
│       └── upload_security.go
├── templates/
│   ├── layouts/
│   │   └── base.html
│   └── public/
│       ├── index.html
│       ├── article.html
│       └── category.html
├── client/
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
│           └── api.js
├── public/
│   ├── css/
│   ├── js/
│   ├── admin/
│   └── uploads/
└── migrations/
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
  max_age: 86400
  idle_timeout: 7200

rate_limit:
  login_ip_window_sec: 600
  login_ip_max_attempts: 20
  login_user_fail_threshold: 5
  login_user_cooldown_sec: 900
  like_ip_window_sec: 60
  like_ip_max_requests: 60
  like_article_window_sec: 600
  like_article_max_actions: 20

upload:
  dir: "public/uploads"
  max_size: 5242880
  allowed_types:
    - image/jpeg
    - image/png
    - image/gif
    - image/webp
  allow_svg: false
  reencode: true

admin:
  init_username: "admin"
  init_password: "<初始密码>"
```

### 安全响应头基线

所有公开页、后台页和静态资源响应默认增加以下安全响应头：

- `Content-Security-Policy`
  - 公开页默认禁止内联脚本，限制资源来源为本站与明确白名单
  - 后台页使用更严格 CSP，禁止被第三方 frame 嵌入
- `X-Content-Type-Options: nosniff`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `X-Frame-Options: DENY`
  - 或在 CSP 中使用 `frame-ancestors 'none'`

### 开发环境

- 确保 Redis 已启动
- 后端：`go run main.go`
- 前端：`cd client && npm run dev`

### 生产构建

- 前端：`cd client && npm run build`（输出到 `public/admin/`）
- 后端：`go build -o blogWeb .`
- 运行：`./blogWeb`
- 使用 `pm2` 或 `systemd` 管理进程（单实例部署）
- 前置反向代理需确保上传目录只做静态托管，不执行脚本

### 初始化流程

1. 首次启动运行数据库迁移脚本
2. 检查是否存在 admin 用户，不存在则根据 `config.yaml` 创建
3. 连接 Redis，连接失败则拒绝启动
4. 首次收到公开请求时，按需为匿名访客签发 `anonymous_id` cookie

### 备份策略

- SQLite：使用 `.backup` 命令热备份
- Redis：需启用 AOF `everysec` 或同等级持久化策略；若未启用可靠持久化，Redis 重启后会话可能丢失
- 上传文件：定期备份 `public/uploads/` 目录

### 安全日志

- 记录后台登录成功 / 失败 / 冷却命中事件
- 记录点赞限流命中事件
- 记录上传校验失败事件
- 记录后台关键写操作审计日志
