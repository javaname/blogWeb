import enUS from './locales/en-US';
import zhCN from './locales/zh-CN';

export const DEFAULT_LOCALE = 'zh-CN';

export const LOCALES = ['zh-CN', 'en-US'];

export const messages = {
  'zh-CN': zhCN,
  'en-US': enUS,
};

export const LANGUAGE_OPTIONS = LOCALES.map((locale) => ({
  value: locale,
  label: messages[locale]?.meta?.languageName || locale,
}));
