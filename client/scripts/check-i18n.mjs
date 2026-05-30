import { parse } from '@babel/parser';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import enUS from '../src/i18n/locales/en-US.js';
import zhCN from '../src/i18n/locales/zh-CN.js';

const locales = {
  'zh-CN': zhCN,
  'en-US': enUS,
};

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const sourceRoot = path.join(projectRoot, 'src');

function flattenKeys(value, prefix = '') {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return prefix ? [prefix] : [];
  }

  return Object.entries(value).flatMap(([key, child]) => {
    const nextPrefix = prefix ? `${prefix}.${key}` : key;
    return flattenKeys(child, nextPrefix);
  });
}

const keySets = Object.fromEntries(
  Object.entries(locales).map(([locale, messages]) => [locale, new Set(flattenKeys(messages))]),
);

const [baseLocale, baseKeys] = Object.entries(keySets)[0];
let hasFailure = false;

for (const [locale, keys] of Object.entries(keySets)) {
  if (locale === baseLocale) {
    continue;
  }

  const missing = [...baseKeys].filter((key) => !keys.has(key));
  const extra = [...keys].filter((key) => !baseKeys.has(key));

  if (missing.length || extra.length) {
    hasFailure = true;
    console.error(`i18n key mismatch: ${baseLocale} vs ${locale}`);
    if (missing.length) {
      console.error(`Missing in ${locale}:`);
      missing.forEach((key) => console.error(`  - ${key}`));
    }
    if (extra.length) {
      console.error(`Extra in ${locale}:`);
      extra.forEach((key) => console.error(`  - ${key}`));
    }
  }
}

function listSourceFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const nextPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      return listSourceFiles(nextPath);
    }
    return /\.(jsx?|tsx?)$/.test(entry.name) ? [nextPath] : [];
  });
}

function hasHumanText(value) {
  const normalized = value.replace(/\s+/g, ' ').trim();
  return /[\u4e00-\u9fff]/.test(normalized) || /[A-Za-z].*\s|\s.*[A-Za-z]/.test(normalized);
}

function textFromJSXText(value) {
  return value.replace(/\s+/g, ' ').trim();
}

function getJSXAttribute(openingElement, name) {
  return openingElement?.attributes?.find((attr) => attr.type === 'JSXAttribute' && attr.name?.name === name);
}

function getStaticAttributeValue(openingElement, name) {
  const attr = getJSXAttribute(openingElement, name);
  return attr?.value?.type === 'StringLiteral' ? attr.value.value : '';
}

function isMaterialIconText(parent) {
  if (parent?.type !== 'JSXElement') {
    return false;
  }
  return getStaticAttributeValue(parent.openingElement, 'className').split(/\s+/).includes('material-symbols-outlined');
}

function isTranslationKeyCall(node, parent) {
  return parent?.type === 'CallExpression' && parent.callee?.type === 'Identifier' && parent.callee.name === 't' && parent.arguments?.[0] === node;
}

function isImportSource(parent) {
  return parent?.type === 'ImportDeclaration' || parent?.type === 'ExportNamedDeclaration' || parent?.type === 'ExportAllDeclaration';
}

function isVisibleStringAttribute(parent) {
  return parent?.type === 'JSXAttribute' && ['alt', 'aria-label', 'placeholder', 'title'].includes(parent.name?.name);
}

function reportHardcoded(file, node, value) {
  const line = node.loc?.start?.line ?? 0;
  const relative = path.relative(projectRoot, file).replace(/\\/g, '/');
  console.error(`${relative}:${line}: hardcoded UI text "${value}"`);
  hasFailure = true;
}

function traverse(node, parent, file) {
  if (!node || typeof node !== 'object') {
    return;
  }

  if (node.type === 'JSXText') {
    const text = textFromJSXText(node.value);
    if (text && hasHumanText(text) && !isMaterialIconText(parent)) {
      reportHardcoded(file, node, text);
    }
    return;
  }

  if (node.type === 'StringLiteral') {
    if (parent?.type === 'JSXAttribute' && !isVisibleStringAttribute(parent)) {
      // Non-visible attributes include className, type, href, SVG path data, etc.
    } else if (isVisibleStringAttribute(parent) && hasHumanText(node.value)) {
      reportHardcoded(file, node, node.value);
    } else if (!isImportSource(parent) && !isTranslationKeyCall(node, parent) && hasHumanText(node.value)) {
      reportHardcoded(file, node, node.value);
    }
  }

  for (const [key, value] of Object.entries(node)) {
    if (key === 'loc' || key === 'start' || key === 'end') {
      continue;
    }
    if (Array.isArray(value)) {
      value.forEach((child) => traverse(child, node, file));
    } else if (value && typeof value === 'object') {
      traverse(value, node, file);
    }
  }
}

const filesToScan = listSourceFiles(sourceRoot).filter((file) => {
  const normalized = file.replace(/\\/g, '/');
  return /\/src\/(components|pages)\//.test(normalized);
});

for (const file of filesToScan) {
  const source = fs.readFileSync(file, 'utf8');
  const ast = parse(source, {
    sourceType: 'module',
    plugins: ['jsx'],
    errorRecovery: false,
  });
  traverse(ast, null, file);
}

if (hasFailure) {
  process.exit(1);
}

console.log(`i18n keys and UI literals OK (${Object.keys(locales).join(', ')})`);
