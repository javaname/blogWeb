# 管理后台 CSS-only 动效修复设计

## 背景

当前 React 管理后台已经实现了主要页面和后台数据链路，但页面呈现接近静态布局。与 Stitch 原型相比，差距集中在管理后台的运动质感：页面进入没有节奏，卡片、表格、图表、顶部面板和登录/注册状态缺少可感知反馈，导致界面显得机械、割裂。

本轮用户确认的范围为 **A：管理后台全量动效修复**。初始实现路径为 **CSS-only 补动效**；验收后发现该路径缺少 JS 驱动的路由/内容重排反馈，因此实现升级为 **CSS 动效层 + 轻量 JS 路由动效 Hook**。

## 目标

- 在不新增动画库、不新增外部依赖的前提下，补齐管理后台关键页面的动效观感。
- 使用少量 JS 驱动路由切换和内容重排的动画重放，避免页面只在首次挂载时出现静态 CSS 入场。
- 最大限度复用现有 React 结构和 CSS class，只在必要处增加语义 class。
- 让后台页面具备接近原型的编辑工作台质感：克制、清晰、有层级，而不是炫技式动效。
- 尊重 `prefers-reduced-motion`，为减少动画偏好的用户关闭非必要运动。

## 非目标

- 不引入 Framer Motion、GSAP 等动画库。
- 不实现复杂 JS timeline、滚动驱动动画或物理动画；JS 仅负责路由切换时标记动效状态和注入 stagger 顺序变量。
- 不重做整体视觉设计、信息架构或 API 数据结构。
- 不覆盖公开博客站点页面；本轮只覆盖 `/admin` 管理后台。

## 实现范围

### 登录页

- 页面视觉区和表单区分段入场。
- 登录卡片上浮进入。
- 登录/注册 tab 切换时，表单内容淡入并轻微上移。
- 错误、成功提示使用轻微 slide-in，强化状态反馈。

### 后台 Shell

- 侧栏品牌、导航项、底部用户卡分段进入。
- 当前导航状态具备背景层级、icon 微位移和 hover/pressed 反馈。
- 顶部通知、帮助、头像、语言和主题按钮具备 hover/pressed 反馈。
- 通知/帮助面板使用 scale + fade 展开，营造 overlay 层级。

### 控制台

- 标题、统计卡、图表区域、活动流 stagger 进入。
- 统计卡 hover lift，阴影和边框强调可交互层级。
- 图表柱形从底部增长；趋势线通过 `stroke-dasharray` / `stroke-dashoffset` 绘制。
- 活动项依次淡入，减少列表突然出现的静态感。

### 列表和管理页面

- 表格行、列表卡片和媒体卡顺序浮入。
- 行 hover 保持克制背景变化，操作按钮 icon 有轻微位移或颜色反馈。
- 空状态、加载状态淡入。

### 文章编辑页

- 主编辑区、侧栏设置卡、封面预览分层进入。
- 编辑器工具栏按钮、标签、上传卡、封面图 hover/pressed 反馈。
- 开关和保存按钮状态有明确视觉反馈。

## CSS 架构

新增或扩展以下 CSS 层：

- Motion tokens：统一时长、缓动、stagger 延迟、hover lift 阴影。
- Keyframes：`admin-page-enter`、`admin-card-enter`、`admin-panel-pop`、`admin-form-swap`、`admin-chart-grow`、`admin-line-draw`、`admin-toast-in`。
- Global selectors：针对 `.admin-page`、`.admin-sidebar`、`.admin-panel`、`.admin-stat-card`、`.admin-table`、`.login-page` 等现有结构添加动效。
- Reduced motion：`@media (prefers-reduced-motion: reduce)` 下关闭动画、过渡和 transform。
- JS route motion：`useAdminRouteMotion(pathname)` 在路由变化后使用 `requestAnimationFrame` 标记 `.admin-canvas.is-motion-enter`、设置 `data-route-motion`，并为内容节点写入 `--motion-order`，让标题、卡片、面板、列表行、媒体卡等按顺序重播入场动效。

优先通过现有选择器实现；如果现有结构无法稳定定位某类元素，允许在 JSX 中增加少量语义 class，例如用于图表 SVG 或页面分区的 class。

## 测试与验收

### 自动化检查

先扩展 `client/scripts/check-ui-completeness.mjs` 或新增同类检查，验证：

- `client/src/styles.css` 包含 motion tokens。
- 存在关键 keyframes。
- 存在 `prefers-reduced-motion` 降级。
- 登录页、Shell、控制台、列表/表格、编辑页关键选择器均有动画或过渡覆盖。
- 存在 JS route motion hook，且 AppShell 已接入；检查 `requestAnimationFrame`、`data-route-motion`、`is-motion-enter`、`--motion-order`。

必须先运行检查并看到失败，再实现样式修复。

### 验证命令

```powershell
npm --prefix client run check:ui
npm --prefix client run build
```

### 视觉验收

- 打开 `http://localhost:5173/admin/`。
- 登录页、控制台、列表页、编辑页在进入和交互时应出现明显但克制的入场、层级、hover、图表绘制反馈。
- 开启系统减少动画偏好时，页面应无明显位移和长动画。

## 风险与取舍

- CSS-only 方案能显著改善静态观感，但不等同于真正 JS timeline 动画。
- 纯选择器 stagger 对动态列表数量有限制；必要时通过 `nth-child` 覆盖常见数量。
- 若后续仍需要更接近原型或更强交互编排，应升级为轻量 JS + CSS Motion Layer。
