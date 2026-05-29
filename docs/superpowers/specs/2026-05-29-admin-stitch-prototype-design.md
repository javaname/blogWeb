# 后台管理端 Stitch 原型落地设计

日期：2026-05-29

## 背景

当前 Web 端位于 `client/src`，采用 React 18、Vite、JSX、Semi UI 相关依赖。现有前端主要覆盖后台管理端，包括登录、控制台、文章、分类、评论、文章编辑和系统设置页面，并已有 i18n 上下文、语言切换组件和中英文文案文件。

Stitch 项目 `Full-stack Blog System` 提供了 `Ink & Insight Admin` 后台原型，包含管理员登录、后台控制台、文章管理、分类管理、评论管理、发布文章、系统设置等页面。本次工作按该后台原型调整现有 React 前端，不切换技术栈，不引入 Rust，不接入前台博客访问页。

## 目标

- 使用现有 React + Vite + JSX 技术栈实现后台管理端视觉升级。
- 对齐 Stitch 后台原型的整体结构、视觉语言和页面密度。
- 保留现有 i18n 能力和语言切换按钮。
- 新增前端界面风格切换按钮，默认支持浅色与深色主题切换。
- 尽量沿用现有路由、API 调用、鉴权逻辑和页面状态，避免重写业务逻辑。
- 保持响应式后台体验，确保桌面端优先，移动端可用。

## 非目标

- 不把前台博客访问页接入 React。
- 不重构后端 API。
- 不切换为 TypeScript 或 Rust。
- 不引入新的大型 UI 框架。
- 不重新设计 i18n 数据结构，只补齐新增 UI 文案。

## 原型映射

本次按 Stitch 后台页面映射到现有路由：

| Stitch 页面 | 现有路由 | 现有文件 |
| --- | --- | --- |
| 管理员登录 | `/login` | `client/src/pages/Login.jsx` |
| 后台控制台 | `/dashboard` | `client/src/pages/Dashboard.jsx` |
| 文章管理 | `/posts` | `client/src/pages/Posts.jsx` |
| 发布文章 | `/articles/new` | `client/src/pages/ArticleEdit.jsx` |
| 编辑文章 | `/articles/:id` | `client/src/pages/ArticleEdit.jsx` |
| 分类管理 | `/categories` | `client/src/pages/Categories.jsx` |
| 评论管理 | `/comments` | `client/src/pages/Comments.jsx` |
| 系统设置 | `/settings` | `client/src/pages/Settings.jsx` |

## 视觉设计

后台壳采用固定左侧导航、顶部工具栏和主内容画布：

- 左侧导航宽度维持约 280px，突出品牌、主要模块入口、新建文章入口和当前用户信息。
- 顶部工具栏放置语言切换、主题切换、通知、帮助和账号入口。
- 主内容区使用最大宽度约 1200px 的居中画布，页面之间保持一致的标题、说明、操作区和内容面板结构。
- 内容面板采用白色背景、细边框、低阴影和 8px 到 16px 圆角，避免过度装饰。
- 主操作使用蓝色，次级操作使用边框按钮，危险操作使用红色文本或图标状态。
- 表格和列表保持后台工具的扫描效率，避免营销式大卡片堆叠。

## 主题切换

新增主题上下文 `ThemeContext` 或等价轻量实现，职责如下：

- 维护当前主题：`light`、`dark`。
- 从 `localStorage` 读取并持久化选择。
- 在 `document.documentElement` 上写入 `data-theme`。
- 暴露 `theme`、`setTheme`、`toggleTheme` 给组件使用。

新增 `ThemeSwitcher` 组件：

- 放在 `AppShell` 顶栏的 `LanguageSwitcher` 旁边。
- 登录页也显示该按钮，入口体验一致。
- 按钮使用图标或简短文本，包含可访问的 `aria-label`。
- 新增文案进入 `zh-CN.js` 和 `en-US.js`，继续使用 `useI18n().t()`。

CSS 改为以变量驱动主题：

- 在 `:root` 定义浅色主题变量。
- 在 `[data-theme='dark']` 定义深色主题变量。
- 页面、按钮、表格、表单、卡片、状态标签逐步从硬编码色值迁移到变量。

## i18n 保留策略

现有 `I18nProvider`、`LanguageSwitcher`、语言包和 `check:i18n` 保持为主路径。

新增或调整的界面文案必须：

- 使用 `t('...')` 获取，不直接硬编码用户可见文案。
- 同步写入 `zh-CN.js` 与 `en-US.js`。
- 通过 `npm run check:i18n` 验证 key 一致性。

## 组件边界

建议新增或整理的组件：

- `ThemeProvider`：主题状态和持久化。
- `ThemeSwitcher`：顶栏/登录页主题切换按钮。
- `AppShell`：继续承载导航、顶栏、通知、帮助和用户入口。
- 页面组件：保持现有文件边界，重点调整布局结构和 className。

不建议把所有页面合并成单个大组件，也不建议从 Stitch HTML 直接复制整页结构。实现应以现有 React 数据流为主，吸收原型布局和视觉规则。

## 数据流

现有数据流保持不变：

- 登录页继续通过 `AuthContext` 处理登录、注册或验证码相关流程。
- 后台页面继续通过 `utils/adminApi.js` 与后端接口通信。
- i18n 继续通过 `I18nContext` 注入。
- 主题状态只影响展示层，不影响 API 请求或业务数据。

## 错误与空状态

页面升级时同步检查以下状态是否符合后台原型风格：

- 加载中：使用轻量 spinner 或内联状态。
- 空列表：使用面板内空状态文本，不用大面积插画。
- 表单错误：沿用当前错误处理，视觉上使用红色提示。
- API 失败：保留现有 i18n 错误消息解析。

## 响应式要求

- 桌面端优先对齐 1280px 及以上后台原型。
- 960px 以下左侧导航改为顶部堆叠或静态块，避免遮挡内容。
- 表格在窄屏退化为单列信息块，操作按钮左对齐。
- 顶栏按钮不换行遮挡，必要时减小间距。

## 验证方案

实现前建立基线：

- `npm run check:i18n`
- `npm run check:ui`
- `npm run build`

实现后重复运行以上命令。若当前项目没有常规单元测试框架，本次不新增测试框架；以现有检查脚本和构建作为验证闭环。若后续需要更强保障，可单独引入 Vitest/React Testing Library。

## 风险

- 现有 CSS 文件较大，主题变量化可能触及较多选择器，需要控制改动范围。
- Stitch 原型包含前台页面，本次只做后台，需避免误接前台路由。
- 深色主题需要检查表格、输入框、状态标签和登录页的对比度。
- 当前仓库存在较多未跟踪文件，提交时必须只纳入本规格文档和后续明确修改文件。

## 验收标准

- 后台主要页面视觉与 Stitch `Ink & Insight Admin` 原型一致性明显提升。
- 语言切换按钮仍可使用，且新增文案中英文完整。
- 顶栏和登录页具备界面风格切换按钮。
- 切换主题后刷新页面仍保持用户选择。
- 后台路由、鉴权和 API 行为不回退。
- `check:i18n`、`check:ui`、`build` 通过。
