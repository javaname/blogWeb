# 博客剩余功能补全设计

## 背景

当前博客前台页面已经露出订阅、收藏、关注作者和评论回复入口，但部分入口仍只是本地提示。后台页面已有仪表盘和设置页，但前端仍偏静态展示。目标是把这些可见功能补成最小可用闭环。

## 方案

采用轻量持久化方案。读者身份继续使用现有 `anonymous_id` Cookie；后端新增订阅、收藏、关注、评论回复所需的数据结构和接口；前端公共脚本将按钮改为真实请求并显示状态。后台 React 页面改为读取已有 admin API，并让设置页可以编辑和保存站点标题、描述、Base URL。

## 后端

- 新增模型：
  - `Subscription`：邮箱、匿名访客、状态和时间。
  - `Bookmark`：文章、匿名访客、时间，唯一约束防重复。
  - `AuthorFollow`：作者、匿名访客、时间，唯一约束防重复。
- 扩展评论：
  - `Comment.ParentID` 支持回复。
  - 公共评论响应包含 `parent_id` 和 `replies`。
- 新增公共接口：
  - `POST /api/newsletter/subscribe`
  - `POST /api/articles/:slug/bookmark`
  - `POST /api/authors/:id/follow`
- 继续使用现有后台接口：
  - `GET /api/admin/dashboard`
  - `GET/PUT /api/admin/settings`

## 前端

- 公共页面：
  - 订阅表单提交到真实接口。
  - 收藏按钮切换收藏状态。
  - 关注按钮切换作者关注状态。
  - 回复按钮填充评论表单的父评论 ID 并滚动到表单。
- 后台：
  - Dashboard 读取真实统计、趋势和活动。
  - Settings 读取接口数据，允许编辑站点基础信息并保存。

## 测试

- 后端先补 handler/service 测试并确认失败，再实现。
- 前端补 UI 完整性检查，确认占位提示不再覆盖真实功能入口。
- 完整验证包括 `go test ./...`、`npm run check:i18n`、`npm run check:ui`、`npm run build`。
