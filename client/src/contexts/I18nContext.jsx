import { createContext, useContext, useEffect, useMemo, useState } from 'react';
import {
  DEFAULT_LOCALE,
  LANGUAGE_OPTIONS,
  LOCALES,
} from '../i18n/messages';
import { hasMessage, readMessage, translateMessage } from '../i18n/translate';
import { configureApiMessages } from '../utils/api';

const LOCALE_STORAGE_KEY = 'blog_admin_locale';

const I18nContext = createContext(null);

function normalizeLocale(locale) {
  return LOCALES.includes(locale) ? locale : DEFAULT_LOCALE;
}

function detectLocale() {
  try {
    const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
    if (LOCALES.includes(stored)) {
      return stored;
    }
  } catch {
    // Ignore storage access failures and fall back to browser language.
  }

  return DEFAULT_LOCALE;
}

export function I18nProvider({ children }) {
  const [locale, setLocaleState] = useState(() => normalizeLocale(detectLocale()));

  useEffect(() => {
    document.documentElement.lang = locale;
    document.title =
      readMessage(locale, 'meta.adminTitle') ??
      readMessage(DEFAULT_LOCALE, 'meta.adminTitle');

    try {
      localStorage.setItem(LOCALE_STORAGE_KEY, locale);
    } catch {
      // Persisting the choice is best effort only.
    }
  }, [locale]);

  const value = useMemo(() => {
    function t(key, values) {
      return translateMessage(locale, key, values);
    }

    return {
      locale,
      dateLocale: locale,
      languageOptions: LANGUAGE_OPTIONS,
      setLocale: (nextLocale) => setLocaleState(normalizeLocale(nextLocale)),
      t,
    };
  }, [locale]);

  useEffect(() => {
    function translateIfExists(key, values) {
      return hasMessage(value.locale, key) ? value.t(key, values) : '';
    }

    configureApiMessages({
      resolveErrorMessage: (payload, status) => {
        const code = typeof payload?.code === 'string' ? payload.code : '';
        if (code) {
          const codeMessage = translateIfExists(`errors.codes.${code}`);
          if (codeMessage) {
            return codeMessage;
          }
        }

        const statusMessage = translateIfExists(`errors.status.${status}`);
        if (statusMessage) {
          return statusMessage;
        }

        return value.t('errors.requestFailed', { status });
      },
    });

    return () => configureApiMessages({});
  }, [value]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within I18nProvider');
  }
  return context;
}
