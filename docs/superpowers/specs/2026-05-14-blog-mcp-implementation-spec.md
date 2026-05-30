# 博客系统 MCP 实施规格

> 基于 `2026-05-13-blog-design-v6.md` 拆解，作为 MCP Server 开发阶段的接口、权限、错误码与测试规格。

---

## 一、适用范围

本文只定义当前博客项目作为 MCP Server 时的实现细节，包括：

- resources
- tools
- prompts
- scope 权限矩阵
- token 与 HTTP 鉴权
- 参数 schema
- 错误码
- 审计与限流
- 测试用例

本文不改变 `v6` 的主设计结论：

- MCP 层只做协议适配。
- 写能力必须复用 `service` 层。
- 不开放任意 SQL、任意文件系统访问、任意 Shell 执行。
- 首版不开放删除类工具。
- HTTP MCP 默认无状态。

---

## 二、传输模式

### 2.1 `stdio`

启动命令：

```text
blogWeb serve-mcp --transport=stdio
```

要求：

- `stdout` 只能输出 MCP 协议消息。
- 日志、错误栈、调试信息只能写入 `stderr`。
- 默认只开放只读能力。
- 写能力必须显式开启 `mcp.stdio_write_enabled=true`。

### 2.2 `Streamable HTTP`

启动命令：

```text
blogWeb serve-mcp --transport=http
```

默认配置：

```yaml
mcp:
  http_enabled: false
  http_addr: "127.0.0.1:3001"
  http_path: "/mcp"
  auth_mode: "pre_shared_token"
  stateless_http: true
  protocol_versions:
    - "2025-11-25"
```

要求：

- 默认采用 POST + JSON 响应，不启用 SSE 推送。
- POST 请求 `Content-Type` 必须为 `application/json`。
- POST 请求 `Accept` 至少包含 `application/json`；为兼容标准客户端，建议同时包含 `application/json, text/event-stream`。
- JSON-RPC request 返回 `Content-Type: application/json` 的单个 JSON-RPC 响应。
- JSON-RPC notification / response 被接受后返回 `202 Accepted`，不返回响应体。
- 未启用 SSE 时，GET 返回 `405 Method Not Allowed`。
- 请求可携带 `MCP-Protocol-Version`，服务端只接受配置中的支持版本。
- 必须校验 Bearer Token。
- 必须校验 Origin，除非明确关闭 `require_origin_check`。
- 不复用后台 session cookie。

---

## 三、权限矩阵

| 能力 | 读公开文章 | 读分类 | 读草稿 | 写草稿 | 发布/撤回 | 上传 | 写分类 |
|------|------------|--------|--------|--------|-----------|------|--------|
| `blog.read` | 是 | 否 | 否 | 否 | 否 | 否 | 否 |
| `blog.category.read` | 否 | 是 | 否 | 否 | 否 | 否 | 否 |
| `blog.draft.write` | 否 | 否 | 是 | 是 | 否 | 否 | 否 |
| `blog.publish` | 否 | 否 | 否 | 否 | 是 | 否 | 否 |
| `blog.upload` | 否 | 否 | 否 | 否 | 否 | 是 | 否 |
| `blog.category.write` | 否 | 否 | 否 | 否 | 否 | 否 | 是 |

规则：

- scope 不自动继承。
- 多 scope 通过 token 显式绑定。
- 工具需要多个权限时，必须同时满足。
- 后台 admin session 不能替代 MCP scope。
- scope 不足时必须返回明确的 `forbidden_scope`，并在 HTTP 场景下返回 `WWW-Authenticate` challenge。

---

## 四、Resources 规格

### 4.1 `blog://site/meta`

scope：`blog.read`

返回：

```json
{
  "title": "个人博客",
  "description": "站点描述",
  "base_url": "https://example.com",
  "version": "v6"
}
```

### 4.2 `blog://categories`

scope：`blog.category.read`

返回：

```json
{
  "list": [
    {
      "id": 1,
      "name": "技术",
      "slug": "tech",
      "sort_order": 0,
      "article_count": 5
    }
  ]
}
```

### 4.3 `blog://articles/{slug}`

scope：`blog.read`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| slug | string | 是 | 仅允许合法 slug 字符，最大 160 字符 |

返回：

```json
{
  "id": 1,
  "title": "文章标题",
  "slug": "article-slug",
  "content_html": "<p>已清洗 HTML</p>",
  "excerpt": "摘要",
  "category": { "id": 1, "name": "技术", "slug": "tech" },
  "is_pinned": false,
  "published_at": "2026-05-13T10:00:00Z",
  "updated_at": "2026-05-13T12:00:00Z"
}
```

要求：

- 只返回公开可见文章。
- 草稿、未来文章、已删除文章返回 `not_found`。
- `content_html` 必须是 sanitizer 后结果。

### 4.4 `blog://drafts/{id}`

scope：`blog.draft.write`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| id | integer | 是 | 正整数 |

返回：

```json
{
  "id": 1,
  "title": "草稿标题",
  "slug": "draft-title",
  "content": "# Markdown 原文",
  "status": "draft",
  "category_id": 1,
  "cover_image": "",
  "is_pinned": false,
  "created_at": "2026-05-13T10:00:00Z",
  "updated_at": "2026-05-13T12:00:00Z"
}
```

要求：

- 该资源可返回 Markdown 原文，但必须要求写权限。
- 审计日志只记录摘要，不记录完整正文。

### 4.5 `blog://categories/{slug}/articles`

scope：`blog.read`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| slug | string | 是 | 分类 slug，最大 160 字符 |

返回：

```json
{
  "category": { "id": 1, "name": "技术", "slug": "tech" },
  "list": [
    {
      "id": 1,
      "title": "文章标题",
      "slug": "article-slug",
      "excerpt": "摘要",
      "published_at": "2026-05-13T10:00:00Z"
    }
  ]
}
```

---

## 五、Tools 规格

工具返回值以下列 JSON 为逻辑结构，实际 MCP 响应由 adapter 包装为协议要求的 content。

### 5.1 `list_articles`

scope：`blog.read`

参数：

| 字段 | 类型 | 必填 | 默认 | 约束 |
|------|------|------|------|------|
| cursor | string | 否 | 空 | 最大 1024 字符 |
| category | string | 否 | 空 | 分类 slug |
| limit | integer | 否 | 12 | 1-50 |

返回：与公开 API `GET /api/articles` 一致。

### 5.2 `get_article`

scope：`blog.read`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| slug | string | 是 | 最大 160 字符 |

返回：与 `blog://articles/{slug}` 一致。

### 5.3 `list_categories`

scope：`blog.category.read`

参数：无。

返回：与 `blog://categories` 一致。

### 5.4 `preview_markdown`

scope：`blog.draft.write`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| content | string | 是 | 最大 200000 字符 |

返回：

```json
{
  "content_html": "<p>已清洗 HTML</p>",
  "excerpt": "纯文本摘要"
}
```

要求：

- 不落库。
- 必须复用 Markdown renderer + sanitizer。
- 禁止返回未清洗 HTML。

### 5.5 `create_article_draft`

scope：`blog.draft.write`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| title | string | 是 | 1-120 字符 |
| content | string | 是 | 最大 200000 字符 |
| category_id | integer | 否 | 正整数 |
| cover_image | string | 否 | 站内上传路径 |
| is_pinned | boolean | 否 | 默认 false |

返回：

```json
{
  "id": 12,
  "slug": "article-title",
  "status": "draft"
}
```

要求：

- 默认创建 `draft`。
- slug 生成必须避开当前 slug 与历史 slug。
- `cover_image` 如存在，只能引用 `/uploads/YYYY/MM/{uuid}.{ext}` 格式的站内资源，禁止外部 URL、绝对磁盘路径和 `..` 路径穿越。

### 5.6 `update_article`

scope：`blog.draft.write`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| id | integer | 是 | 正整数 |
| title | string | 否 | 1-120 字符 |
| content | string | 否 | 最大 200000 字符 |
| category_id | integer | 否 | 正整数或 null |
| cover_image | string | 否 | 站内上传路径 |
| is_pinned | boolean | 否 | true / false |

返回：

```json
{
  "id": 12,
  "slug": "new-article-title",
  "updated_at": "2026-05-14T10:00:00Z"
}
```

要求：

- 标题变化导致 slug 变化时，旧 slug 必须写入 `slug_history`。
- 更新已发布文章时，不得绕过 `published_at` 规则。
- `cover_image` 如存在，只能引用 `/uploads/YYYY/MM/{uuid}.{ext}` 格式的站内资源，禁止外部 URL、绝对磁盘路径和 `..` 路径穿越。

### 5.7 `publish_article`

scope：`blog.publish`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| id | integer | 是 | 正整数 |
| published_at | string | 否 | RFC3339 |

返回：

```json
{
  "id": 12,
  "status": "published",
  "published_at": "2026-05-14T10:00:00Z"
}
```

要求：

- 未传 `published_at` 时使用当前时间。
- 未来发布时间文章在公开侧不可见。

### 5.8 `unpublish_article`

scope：`blog.publish`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| id | integer | 是 | 正整数 |

返回：

```json
{
  "id": 12,
  "status": "draft"
}
```

要求：

- 切回草稿后公开侧立即不可见。
- `published_at` 保留原值。

### 5.9 `upload_image`

scope：`blog.upload`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| filename | string | 是 | 仅用于审计与扩展名参考，不作为最终文件名 |
| mime_type | string | 是 | `image/jpeg` / `image/png` / `image/gif` / `image/webp` |
| content_base64 | string | 是 | 解码后最大 5MB |

返回：

```json
{
  "url": "/uploads/2026/05/a1b2c3d4.jpg",
  "filename": "a1b2c3d4.jpg",
  "mime_type": "image/jpeg",
  "size": 123456
}
```

要求：

- 不接受本地文件路径参数。
- 不允许服务端按客户端传入路径读取文件。
- 解码后必须走 `v5` 上传安全链路：
  - MIME 魔数校验
  - 禁止 SVG
  - 重建扩展名
  - 推荐重编码
  - 固定 `Content-Type`
  - `nosniff`

### 5.10 `create_category`

scope：`blog.category.write`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| name | string | 是 | 1-40 字符 |
| slug | string | 否 | 最大 160 字符 |
| sort_order | integer | 否 | 默认 0 |

返回：

```json
{
  "id": 3,
  "name": "新分类",
  "slug": "new-category"
}
```

### 5.11 `update_category`

scope：`blog.category.write`

参数：

| 字段 | 类型 | 必填 | 约束 |
|------|------|------|------|
| id | integer | 是 | 正整数 |
| name | string | 否 | 1-40 字符 |
| slug | string | 否 | 最大 160 字符 |
| sort_order | integer | 否 | 非负整数 |

返回：

```json
{
  "id": 3,
  "name": "新名称",
  "slug": "new-slug",
  "sort_order": 1
}
```

---

## 六、Prompts 规格

### 6.1 `draft_article_from_outline`

参数：

| 字段 | 类型 | 必填 |
|------|------|------|
| title | string | 是 |
| outline | string | 是 |
| audience | string | 否 |
| tone | string | 否 |

输出要求：

- 生成适合博客文章的草稿写作提示。
- 不直接创建文章。
- 若需要落库，必须由客户端显式调用 `create_article_draft`。

### 6.2 `seo_review_article`

参数：

| 字段 | 类型 | 必填 |
|------|------|------|
| title | string | 是 |
| content | string | 是 |
| keywords | array<string> | 否 |

输出要求：

- 返回标题、摘要、关键词、结构建议。
- 不修改原文章。

### 6.3 `rewrite_article_summary`

参数：

| 字段 | 类型 | 必填 |
|------|------|------|
| title | string | 是 |
| content | string | 是 |
| target_length | integer | 否 |

输出要求：

- 返回摘要建议。
- 不直接写入 `excerpt`。

---

## 七、错误码

| code | 含义 | 建议映射 |
|------|------|----------|
| `auth_required` | 缺少认证信息 | HTTP 401 |
| `invalid_token` | token 无效或已撤销 | HTTP 401 |
| `forbidden_scope` | scope 不足 | HTTP 403 |
| `invalid_origin` | Origin 不允许 | HTTP 403 |
| `invalid_params` | 参数错误 | HTTP 400 |
| `not_found` | 资源不存在或不可见 | HTTP 404 |
| `conflict` | 业务冲突 | HTTP 409 |
| `payload_too_large` | 请求体过大 | HTTP 413 |
| `unsupported_media_type` | 上传类型不支持 | HTTP 415 |
| `rate_limited` | 命中限流 | HTTP 429 |
| `internal_error` | 服务端错误 | HTTP 500 |

错误响应逻辑结构：

```json
{
  "code": "forbidden_scope",
  "message": "MCP token 缺少 blog.publish 权限",
  "request_id": "req_abc123"
}
```

HTTP 鉴权响应头要求：

```text
WWW-Authenticate: Bearer resource_metadata="https://example.com/.well-known/oauth-protected-resource"
```

scope 不足时：

```text
WWW-Authenticate: Bearer error="insufficient_scope", scope="blog.publish"
```

说明：

- 首版 `auth_mode=pre_shared_token` 可将 `resource_metadata` 指向私有令牌签发说明或未来 OAuth Protected Resource Metadata。
- 若面向公网第三方 MCP 客户端开放，必须实现 OAuth Protected Resource Metadata 与 token audience 校验。
- 预注册 opaque token 只对当前博客实例有效，不接受第三方 access token 直传。

---

## 八、审计字段

每次 MCP 调用写入 `mcp_audit_logs`：

```json
{
  "client_id": 1,
  "transport": "http",
  "action_type": "tool_call",
  "target": "publish_article",
  "scope": "blog.publish",
  "status": "success",
  "request_id": "req_abc123",
  "actor_ip": "127.0.0.1",
  "error_code": "",
  "payload_digest": "sha256:..."
}
```

脱敏要求：

- 不记录明文 token。
- 不记录完整 Markdown 正文。
- 不记录完整图片 base64。
- 不记录密码、session secret、CSRF token。

---

## 九、测试用例

### 9.1 权限测试

- 无 token 调用 HTTP MCP，预期拒绝。
- 无效 token 调用 HTTP MCP，预期拒绝。
- 只读 token 调用 `publish_article`，预期 `forbidden_scope`。
- `blog.upload` token 调用 `create_article_draft`，预期 `forbidden_scope`。
- 401 响应包含 `WWW-Authenticate`。
- scope 不足响应包含 `WWW-Authenticate: Bearer error="insufficient_scope"`。

### 9.2 可见性测试

- `list_articles` 不返回草稿。
- `get_article` 查询未来发布时间文章，预期 `not_found`。
- `blog://drafts/{id}` 在无 `blog.draft.write` 时拒绝。

### 9.3 安全测试

- `preview_markdown` 输入 `<script>`，输出不包含脚本。
- `upload_image` 上传 SVG，预期拒绝。
- `upload_image` 上传 MIME 伪装文件，预期拒绝。
- HTTP MCP 使用非法 Origin，预期拒绝。
- HTTP MCP 使用不支持的 `MCP-Protocol-Version`，预期拒绝。
- HTTP MCP 缺少或错误 `Accept`，预期拒绝或返回协议错误。
- `create_article_draft` / `update_article` 传入外部 `cover_image` URL，预期拒绝。
- `create_article_draft` / `update_article` 传入包含 `..` 的 `cover_image`，预期拒绝。
- 高频调用写工具，预期触发限流。

### 9.4 一致性测试

- MCP 创建草稿后，后台文章列表可见该草稿。
- MCP 发布文章后，公开 API 可查询到该文章。
- MCP 更新标题后，旧 slug 写入 `slug_history`。
- MCP 上传图片后，后台编辑器可引用返回 URL。

---

## 十、实现注意事项

- `upload_image` 不接受本地路径，避免变相文件读取能力。
- 所有 tool 参数必须先经过 schema 校验，再进入 service 层。
- `stdio` 写能力默认关闭。
- HTTP MCP 默认不公网暴露。
- token 与后台账号密码生命周期独立。
- token 哈希推荐使用 `HMAC-SHA256(server_secret, token)` 或同等强度方案，并使用常量时间比较。
- Prompt 输出不得自动触发工具调用。
- MCP SDK 版本应在实现时锁定，并记录其支持的传输能力。
