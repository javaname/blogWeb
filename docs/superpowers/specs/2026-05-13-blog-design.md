# 个人博客系统设计规范

## 概述

本系统为个人文章发布博客，作者可通过 Markdown 编写和发布文章，外部用户可浏览文章、匿名点赞。

## 技术栈

| 层级 | 选型 | 理由 |
|------|------|------|
| 后端语言 | Go 1.22+ | 高性能、单二进制部署 |
| Web 框架 | `gin` | 路由强大、生态丰富 |
| 数据库 | SQLite + GORM | 零配置、单文件、自动迁移 |
| 会话管理 | `gin-contrib/sessions` | 内存存储，个人博客够用 |
| 密码哈希 | `golang.org/x/crypto/bcrypt` | 官方扩展库 |
| Markdown | `goldmark` | Go 生态主流 Markdown 库 |
| 代码高亮 | `goldmark-highlighting` | goldmark 扩展 |
| 图片上传 | Gin 原生 `FormFile` | 无需额外依赖 |
| 模板引擎 | `html/template` | Go 标准库 |
| 管理后台前端 | React + Semi Design | 完整组件库，交互统一 |
| 管理后台构建 | Vite | 现代前端构建工具 |
| 公开页面样式 | Semi Design CSS 变量 | 与后台视觉统一 |

## 数据库设计

### `users` — 用户表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PK AUTOINCREMENT | 自增主键 |
| username | TEXT UNIQUE NOT NULL | 用户名 |
| password | TEXT NOT NULL | bcrypt 哈希密码 |
| role | TEXT NOT NULL DEFAULT 'user' | 'admin' / 'user' |
| created_at | DATETIME | 创建时间 |

### `categories` — 分类表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PK AUTOINCREMENT | 自增主键 |
| name | TEXT UNIQUE NOT NULL | 分类名称 |
| slug | TEXT UNIQUE NOT NULL | URL 友好标识 |
| sort_order | INTEGER DEFAULT 0 | 排序权重 |
| created_at | DATETIME | 创建时间 |

### `articles` — 文章表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PK AUTOINCREMENT | 自增主键 |
| title | TEXT NOT NULL | 文章标题 |
| slug | TEXT UNIQUE NOT NULL | URL 友好标识 |
| content | TEXT NOT NULL | Markdown 原始内容 |
| cover_image | TEXT DEFAULT '' | 封面图路径 |
| excerpt | TEXT DEFAULT '' | 文章摘要，自动截取 |
| category_id | INTEGER FK | 关联分类 |
| status | TEXT DEFAULT 'draft' | 'draft' / 'published' |
| is_pinned | INTEGER DEFAULT 0 | 是否置顶 (0/1) |
| published_at | DATETIME | 发布时间（可手动修改） |
| created_at | DATETIME | 创建时间 |
| updated_at | DATETIME | 更新时间 |

### `likes` — 点赞表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PK AUTOINCREMENT | 自增主键 |
| article_id | INTEGER FK NOT NULL | 关联文章 |
| ip_address | TEXT NOT NULL | 匿名用户 IP |
| user_agent | TEXT DEFAULT '' | 浏览器标识 |
| created_at | DATETIME | 点赞时间 |

唯一约束: (article_id, ip_address) 防止同 IP 重复点赞。

## 路由设计

### 公开页面路由

| 路由 | 处理 | 说明 |
|------|------|------|
| `GET /` | 首页模板 | 最新文章卡片列表，无限滚动 |
| `GET /articles/:slug` | 文章详情模板 | 渲染 Markdown 内容 + 点赞按钮 |
| `GET /categories/:slug` | 分类页面模板 | 特定分类下的文章列表 |
| `GET /api/articles` | JSON | 无限滚动数据，参数 `?page=&category=` |
| `POST /api/articles/:id/like` | JSON | 点赞/取消点赞，返回 `{liked, count}` |

### 管理后台路由（需登录）

| 路由 | 说明 |
|------|------|
| `POST /api/admin/login` | 登录提交，返回 session |
| `POST /api/admin/logout` | 退出登录 |
| `GET /api/admin/articles` | 文章列表（含点赞统计） |
| `POST /api/admin/articles` | 新建文章 |
| `PUT /api/admin/articles/:id` | 更新文章 |
| `DELETE /api/admin/articles/:id` | 删除文章 |
| `GET /api/admin/categories` | 分类列表 |
| `POST /api/admin/categories` | 新建分类 |
| `PUT /api/admin/categories/:id` | 更新分类 |
| `DELETE /api/admin/categories/:id` | 删除分类 |
| `POST /api/admin/upload` | 图片上传 |
| `GET /admin/*` | React SPA 入口 | 管理后台前端页面 |

### 预留路由（后端逻辑就位，前端暂不开放）

| 路由 | 说明 |
|------|------|
| `POST /api/register` | 用户注册 |
| `POST /api/login` | 用户登录 |
| `POST /api/logout` | 用户退出 |

## 前端设计

### 公开页面（Go 模板 + 原生 JS）

- **首页**：时间倒序卡片网格布局，置顶文章置顶显示。无限滚动通过 IntersectionObserver 监听哨兵元素，fetch `/api/articles` 追加渲染
- **文章详情页**：goldmark 渲染 Markdown，highlight.js 代码高亮，底部点赞按钮
- **分类页面**：与首页类似，按分类过滤
- **响应式**：桌面 3 列 → 平板 2 列 → 手机 1 列
- **样式**：引入 Semi Design CSS 变量，自定义样式遵循 Design Tokens 规范

### 管理后台（React SPA + Semi Design）

- **登录页**：Semi Design Form 组件，账号密码登录
- **仪表盘**：Semi Table 展示文章列表（标题、分类、状态、点赞数、发布时间），支持排序筛选
- **文章编辑**：EasyMDE 编辑器集成到 React，支持封面图上传、分类选择、草稿/发布切换、置顶开关、修改发布时间
- **分类管理**：Semi Table + Modal，支持拖拽排序

## 项目结构

```
blogWeb/
├── main.go
├── go.mod
├── .env
├── data/                    # SQLite 数据库文件
├── config/
│   └── config.go            # 配置加载
├── internal/
│   ├── middleware/
│   │   └── auth.go          # 登录验证中间件
│   ├── handler/
│   │   ├── article.go
│   │   ├── category.go
│   │   ├── auth.go
│   │   └── upload.go
│   ├── model/
│   │   ├── user.go
│   │   ├── article.go
│   │   ├── category.go
│   │   └── like.go
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
├── client/                  # React 管理后台
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
│       └── components/
├── public/
│   ├── css/
│   ├── js/
│   ├── admin/               # React 构建产物
│   └── uploads/             # 上传图片
└── utils/
    └── markdown.go
```

## 部署与运维

### 环境变量 (`.env`)

```
PORT=3000
SESSION_SECRET=<随机密钥>
INIT_ADMIN_USERNAME=admin
INIT_ADMIN_PASSWORD=<初始密码>
UPLOAD_DIR=public/uploads
DB_PATH=data/blog.db
```

### 开发环境

- 后端：`go run main.go`
- 前端：`cd client && npm run dev`（Vite 代理 API 到后端）

### 生产构建

- 前端：`cd client && npm run build`（输出到 `public/admin/`）
- 后端：`go build -o blogWeb .`
- 运行：`./blogWeb`
- 使用 `pm2` 或 `systemd` 管理进程

### 初始化流程

1. 首次启动自动创建数据库表（GORM AutoMigrate）
2. 检查是否存在 admin 用户，不存在则根据 `.env` 创建
3. 图片上传限制：单张 5MB，允许 jpg/png/gif/webp

### 备份

SQLite 单文件存储，备份即复制 `data/` 目录。