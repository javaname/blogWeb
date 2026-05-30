import { createContext, useContext, useEffect, useMemo, useState } from 'react';

const THEME_STORAGE_KEY = 'blog_admin_theme';
const THEMES = ['light', 'dark'];
const DEFAULT_THEME = 'light';

const ThemeContext = createContext(null);

function normalizeTheme(theme) {
  return THEMES.includes(theme) ? theme : DEFAULT_THEME;
}

function detectTheme() {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (THEMES.includes(stored)) {
      return stored;
    }
  } catch {
    // Theme persistence is best effort.
  }
  return DEFAULT_THEME;
}

export function ThemeProvider({ children }) {
  const [theme, setThemeState] = useState(() => normalizeTheme(detectTheme()));

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // Ignore storage failures.
    }
  }, [theme]);

  const value = useMemo(() => {
    function setTheme(nextTheme) {
      setThemeState(normalizeTheme(nextTheme));
    }

    function toggleTheme() {
      setThemeState((current) => (current === 'dark' ? 'light' : 'dark'));
    }

    return {
      theme,
      isDark: theme === 'dark',
      setTheme,
      toggleTheme,
    };
  }, [theme]);

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within ThemeProvider');
  }
  return context;
}
