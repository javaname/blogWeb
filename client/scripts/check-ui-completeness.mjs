import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

const filesToScanForMojibake = [
  'templates/_base.html',
  'templates/index.html',
  'templates/article.html',
  'templates/category.html',
  'public/assets/site.js',
  'src/http_public.rs',
  'src/admin_read.rs',
  'src/admin_write.rs',
];

const mojibakePatterns = [
  /鏂/,
  /璇/,
  /鍒/,
  /绮/,
  /鏈/,
  /鎴/,
  /鐧/,
  /闃/,
  /€\?/,
  /厈/,
  /寋/,
];

let hasFailure = false;

function fail(message) {
  hasFailure = true;
  console.error(message);
}

for (const relativePath of filesToScanForMojibake) {
  const absolutePath = path.join(projectRoot, relativePath);
  const source = fs.readFileSync(absolutePath, 'utf8');
  const matched = mojibakePatterns.find((pattern) => pattern.test(source));
  if (matched) {
    fail(`${relativePath}: contains mojibake pattern ${matched}`);
  }
}

const articleEditor = fs.readFileSync(path.join(projectRoot, 'client/src/pages/ArticleEdit.jsx'), 'utf8');
if (!articleEditor.includes('uploadImage')) {
  fail('client/src/pages/ArticleEdit.jsx: cover upload API is not wired');
}
if (!/type=["']file["']/.test(articleEditor)) {
  fail('client/src/pages/ArticleEdit.jsx: cover upload file input is missing');
}

const siteScript = fs.readFileSync(path.join(projectRoot, 'public/assets/site.js'), 'utf8');
if (!siteScript.includes('data-search-form')) {
  fail('public/assets/site.js: public search form behavior is missing');
}
if (!siteScript.includes("new URL('/search'")) {
  fail('public/assets/site.js: public search should navigate to /search');
}
if (!siteScript.includes('/api/newsletter/subscribe')) {
  fail('public/assets/site.js: newsletter subscribe API is not wired');
}
if (!siteScript.includes('/bookmark')) {
  fail('public/assets/site.js: bookmark API is not wired');
}
if (!siteScript.includes('/api/authors/')) {
  fail('public/assets/site.js: author follow API is not wired');
}
if (!siteScript.includes('parent_id')) {
  fail('public/assets/site.js: comment reply parent_id is not wired');
}

const dashboardPage = fs.readFileSync(path.join(projectRoot, 'client/src/pages/Dashboard.jsx'), 'utf8');
if (!dashboardPage.includes('fetchDashboard')) {
  fail('client/src/pages/Dashboard.jsx: dashboard API is not wired');
}

const settingsPage = fs.readFileSync(path.join(projectRoot, 'client/src/pages/Settings.jsx'), 'utf8');
if (!settingsPage.includes('fetchSettings') || !settingsPage.includes('updateSettings')) {
  fail('client/src/pages/Settings.jsx: settings read/update APIs are not wired');
}

const mainEntry = fs.readFileSync(path.join(projectRoot, 'client/src/main.jsx'), 'utf8');
if (!mainEntry.includes('ThemeProvider')) {
  fail('client/src/main.jsx: ThemeProvider is not wired');
}
if (!mainEntry.includes('ToastProvider')) {
  fail('client/src/main.jsx: ToastProvider is not wired');
}

const appShell = fs.readFileSync(path.join(projectRoot, 'client/src/components/AppShell.jsx'), 'utf8');
const loginPage = fs.readFileSync(path.join(projectRoot, 'client/src/pages/Login.jsx'), 'utf8');
const appRoutes = fs.readFileSync(path.join(projectRoot, 'client/src/App.jsx'), 'utf8');
const usersPage = fs.readFileSync(path.join(projectRoot, 'client/src/pages/Users.jsx'), 'utf8');
const adminApi = fs.readFileSync(path.join(projectRoot, 'client/src/utils/adminApi.js'), 'utf8');
if (!appShell.includes('ThemeSwitcher')) {
  fail('client/src/components/AppShell.jsx: theme switcher is missing from admin shell');
}
if (!loginPage.includes('ThemeSwitcher')) {
  fail('client/src/pages/Login.jsx: theme switcher is missing from login page');
}
if (!appShell.includes('useAdminRouteMotion') || !appShell.includes('useAdminRouteMotion()')) {
  fail('client/src/components/AppShell.jsx: JS route motion hook is not wired');
}
if (appShell.includes('useAdminRouteMotion(pathname)')) {
  fail('client/src/components/AppShell.jsx: route motion hook must not subscribe to pathname changes');
}
for (const [route, page, navKey] of [
  ['media', 'Media', 'shell.navMedia'],
  ['users', 'Users', 'shell.navUsers'],
  ['analytics', 'Analytics', 'shell.navAnalytics'],
]) {
  if (!appRoutes.includes(`path="${route}"`) || !appRoutes.includes(`<${page} />`)) {
    fail(`client/src/App.jsx: /${route} route is missing`);
  }
  if (!appShell.includes(navKey) || !appShell.includes(`/${route}`)) {
    fail(`client/src/components/AppShell.jsx: /${route} navigation is missing`);
  }
  const pagePath = path.join(projectRoot, `client/src/pages/${page}.jsx`);
  if (!fs.existsSync(pagePath)) {
    fail(`client/src/pages/${page}.jsx: page is missing`);
    continue;
  }
  const pageSource = fs.readFileSync(pagePath, 'utf8');
  if (!pageSource.includes(`data-page="${route}"`)) {
    fail(`client/src/pages/${page}.jsx: data-page="${route}" hook is missing`);
  }
}
for (const snippet of ['fetchUsers', 'createUser', 'updateUserRole', 'deleteUser']) {
  if (!usersPage.includes(snippet)) {
    fail(`client/src/pages/Users.jsx: ${snippet} is not wired`);
  }
}
for (const snippet of ['data-user-create-form', 'data-user-role-select', 'data-user-delete']) {
  if (!usersPage.includes(snippet)) {
    fail(`client/src/pages/Users.jsx: ${snippet} hook is missing`);
  }
}
if (!usersPage.includes('showAdminToast')) {
  fail('client/src/pages/Users.jsx: success popup toast is not wired');
}
if (!appRoutes.includes('UserDetail') || !appRoutes.includes('path="users/:id"')) {
  fail('client/src/App.jsx: /users/:id detail route is missing');
}
for (const snippet of ['fetchUser', 'updateUser']) {
  if (!adminApi.includes(snippet)) {
    fail(`client/src/utils/adminApi.js: ${snippet} is not wired`);
  }
}
if (!usersPage.includes('/users/${user.id}') || !usersPage.includes('data-user-edit')) {
  fail('client/src/pages/Users.jsx: user detail edit navigation is missing');
}
const userDetailPath = path.join(projectRoot, 'client/src/pages/UserDetail.jsx');
if (!fs.existsSync(userDetailPath)) {
  fail('client/src/pages/UserDetail.jsx: page is missing');
} else {
  const userDetailPage = fs.readFileSync(userDetailPath, 'utf8');
  for (const snippet of [
    'data-page="user-detail"',
    'data-user-detail-form',
    'fetchUser',
    'updateUser',
    'showAdminToast',
    'recent_articles',
    'admin-user-detail-profile',
    'admin-related-articles',
    'users.detailTitle',
    'users.detailSaved',
    'users.relatedArticlesTitle',
  ]) {
    if (!userDetailPage.includes(snippet)) {
      fail(`client/src/pages/UserDetail.jsx: detail page snippet ${snippet} is missing`);
    }
  }
}
for (const snippet of [
  'userDisplayName',
  'formatDateTime',
  'users.accountLine',
  'users.createdAt',
  'users.articleCount',
  'users.noArticles',
  'users.deleteBlockedByArticles',
  'admin-user-articles',
  'disabled={deleteBlocked}',
]) {
  if (!usersPage.includes(snippet)) {
    fail(`client/src/pages/Users.jsx: member detail display snippet ${snippet} is missing`);
  }
}

const toastProviderPath = path.join(projectRoot, 'client/src/components/ToastProvider.jsx');
if (!fs.existsSync(toastProviderPath)) {
  fail('client/src/components/ToastProvider.jsx: global toast provider is missing');
} else {
  const toastProvider = fs.readFileSync(toastProviderPath, 'utf8');
  for (const snippet of ['__BLOG_ADMIN_TOAST__', 'success', 'error', 'info', 'admin-toast-viewport', 'role="status"']) {
    if (!toastProvider.includes(snippet)) {
      fail(`client/src/components/ToastProvider.jsx: ${snippet} is missing`);
    }
  }
}

const baseTemplate = fs.readFileSync(path.join(projectRoot, 'templates/_base.html'), 'utf8');
if (baseTemplate.includes('href="#categories"') || baseTemplate.includes('href="#about"')) {
  fail('templates/_base.html: public navigation should use real /categories and /about routes');
}

const themeContextPath = path.join(projectRoot, 'client/src/contexts/ThemeContext.jsx');
if (!fs.existsSync(themeContextPath)) {
  fail('client/src/contexts/ThemeContext.jsx: theme context is missing');
}

const adminMotionPath = path.join(projectRoot, 'client/src/hooks/useAdminRouteMotion.js');
if (!fs.existsSync(adminMotionPath)) {
  fail('client/src/hooks/useAdminRouteMotion.js: JS admin route motion hook is missing');
} else {
  const adminMotion = fs.readFileSync(adminMotionPath, 'utf8');
  for (const snippet of ['requestAnimationFrame', 'data-route-motion', 'is-motion-enter', '--motion-order', 'prefers-reduced-motion']) {
    if (!adminMotion.includes(snippet)) {
      fail(`client/src/hooks/useAdminRouteMotion.js: ${snippet} is missing`);
    }
  }
  if (/\},\s*\[pathname\]\);/.test(adminMotion)) {
    fail('client/src/hooks/useAdminRouteMotion.js: route motion must not replay on every pathname change');
  }
}

const styles = fs.readFileSync(path.join(projectRoot, 'client/src/styles.css'), 'utf8');
if (!styles.includes("[data-theme='dark']")) {
  fail('client/src/styles.css: dark theme variables are missing');
}
for (const token of ['--color-background', '--color-surface', '--color-primary', '--color-text', '--color-border']) {
  if (!styles.includes(token)) {
    fail(`client/src/styles.css: theme token ${token} is missing`);
  }
}

function requireStyleSnippet(snippet, message) {
  if (!styles.includes(snippet)) {
    fail(`client/src/styles.css: ${message}`);
  }
}

function requireStylePattern(pattern, message) {
  if (!pattern.test(styles)) {
    fail(`client/src/styles.css: ${message}`);
  }
}

for (const token of [
  '--motion-duration-fast',
  '--motion-duration-base',
  '--motion-duration-slow',
  '--motion-ease-standard',
  '--motion-ease-emphasized',
  '--motion-stagger-step',
  '--shadow-motion-lift',
]) {
  requireStyleSnippet(token, `motion token ${token} is missing`);
}

for (const keyframe of [
  'admin-page-enter',
  'admin-card-enter',
  'admin-panel-pop',
  'admin-form-swap',
  'admin-chart-grow',
  'admin-line-draw',
  'admin-toast-in',
]) {
  requireStyleSnippet(`@keyframes ${keyframe}`, `motion keyframe ${keyframe} is missing`);
}

requireStyleSnippet('@media (prefers-reduced-motion: reduce)', 'reduced-motion fallback is missing');
requireStylePattern(/\.login-page__visual[\s\S]*?animation:\s*admin-page-enter/, 'login visual panel entry animation is missing');
requireStylePattern(/\.login-card[\s\S]*?animation:\s*admin-card-enter/, 'login card entry animation is missing');
requireStylePattern(/\.login-card__head[\s\S]*?animation:\s*admin-form-swap/, 'login form swap animation is missing');
requireStylePattern(/\.admin-sidebar__brand[\s\S]*?animation:\s*admin-page-enter/, 'sidebar brand entry animation is missing');
requireStylePattern(/\.admin-nav__item[\s\S]*?animation:\s*admin-card-enter/, 'sidebar nav stagger animation is missing');
requireStylePattern(/\.admin-topbar-panel[\s\S]*?animation:\s*admin-panel-pop/, 'topbar panel pop animation is missing');
requireStylePattern(/\.admin-page__header[\s\S]*?animation:\s*admin-page-enter/, 'admin page header entry animation is missing');
requireStylePattern(/\.admin-stat-card[\s\S]*?animation:\s*admin-card-enter/, 'dashboard stat card entry animation is missing');
requireStylePattern(/\.admin-chart__bars span[\s\S]*?animation:\s*admin-chart-grow/, 'dashboard chart bar growth animation is missing');
requireStylePattern(/\.admin-chart svg path[\s\S]*?animation:\s*admin-line-draw/, 'dashboard chart line draw animation is missing');
requireStylePattern(/\.admin-list-table__row[\s\S]*?animation:\s*admin-card-enter/, 'admin list row entry animation is missing');
requireStylePattern(/\.article-edit[\s\S]*?animation:\s*admin-page-enter/, 'article editor entry animation is missing');
requireStylePattern(/\.article-edit-sidebar[\s\S]*?animation:\s*admin-card-enter/, 'article editor sidebar stagger animation is missing');
requireStylePattern(/\.admin-inline-banner[\s\S]*?animation:\s*admin-toast-in/, 'inline status banner animation is missing');
requireStyleSnippet('@keyframes admin-js-route-enter', 'JS route motion keyframe admin-js-route-enter is missing');
requireStylePattern(/\.admin-canvas\.is-motion-enter[\s\S]*?animation:\s*admin-js-route-enter/, 'JS route motion canvas selector is missing');
requireStylePattern(/\.admin-toast-viewport[\s\S]*?position:\s*fixed/, 'global toast viewport is missing');
requireStylePattern(/\.admin-toast[\s\S]*?animation:\s*admin-toast-in/, 'global toast animation is missing');
requireStylePattern(/\.admin-toast\.is-error[\s\S]*?color:\s*var\(--color-danger-text\)/, 'error toast style is missing');
requireStylePattern(/\.admin-toast\.is-success[\s\S]*?color:\s*var\(--color-success-text\)/, 'success toast style is missing');
requireStylePattern(/\.admin-user-cell__account[\s\S]*?color:\s*var\(--color-text-muted\)/, 'member account metadata style is missing');
requireStylePattern(/\.admin-user-articles[\s\S]*?flex-direction:\s*column/, 'member article association style is missing');
requireStylePattern(/\.admin-icon-button:disabled[\s\S]*?cursor:\s*not-allowed/, 'disabled icon button style is missing');

if (hasFailure) {
  process.exit(1);
}

console.log('UI completeness checks OK');
