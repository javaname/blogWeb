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
  'internal/handler/http.go',
  'internal/handler/presentation.go',
  'internal/service/article.go',
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

const appShell = fs.readFileSync(path.join(projectRoot, 'client/src/components/AppShell.jsx'), 'utf8');
const loginPage = fs.readFileSync(path.join(projectRoot, 'client/src/pages/Login.jsx'), 'utf8');
if (!appShell.includes('ThemeSwitcher')) {
  fail('client/src/components/AppShell.jsx: theme switcher is missing from admin shell');
}
if (!loginPage.includes('ThemeSwitcher')) {
  fail('client/src/pages/Login.jsx: theme switcher is missing from login page');
}

const themeContextPath = path.join(projectRoot, 'client/src/contexts/ThemeContext.jsx');
if (!fs.existsSync(themeContextPath)) {
  fail('client/src/contexts/ThemeContext.jsx: theme context is missing');
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

if (hasFailure) {
  process.exit(1);
}

console.log('UI completeness checks OK');
