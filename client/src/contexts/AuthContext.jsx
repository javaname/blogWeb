import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { fetchCsrfToken, fetchCurrentUser, login as loginApi, logout as logoutApi } from '../utils/adminApi';
import { clearCsrfToken, configureApiHandlers, setCsrfToken } from '../utils/api';

const USER_STORAGE_KEY = 'blog_admin_user';

const AuthContext = createContext(null);

function readStoredUser() {
  try {
    const raw = sessionStorage.getItem(USER_STORAGE_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function persistUser(user) {
  if (user) {
    sessionStorage.setItem(USER_STORAGE_KEY, JSON.stringify(user));
  } else {
    sessionStorage.removeItem(USER_STORAGE_KEY);
  }
}

export function AuthProvider({ children }) {
  const navigate = useNavigate();
  const [user, setUser] = useState(() => readStoredUser());
  const [ready, setReady] = useState(false);

  const applyUser = useCallback((nextUser) => {
    persistUser(nextUser);
    setUser((current) => (JSON.stringify(current) === JSON.stringify(nextUser) ? current : nextUser));
  }, []);

  const refreshCurrentUser = useCallback(async () => {
    const currentUser = await fetchCurrentUser();
    const nextUser = currentUser?.user || null;
    applyUser(nextUser);
    return nextUser;
  }, [applyUser]);

  useEffect(() => {
    const handleUnauthorized = () => {
      persistUser(null);
      setUser(null);
      navigate('/login', { replace: true, state: { reason: 'unauthorized' } });
    };

    const handleForbidden = () => {
      navigate('/login', { replace: true, state: { reason: 'forbidden' } });
    };

    configureApiHandlers({
      onUnauthorized: handleUnauthorized,
      onForbidden: handleForbidden,
    });

    return () => configureApiHandlers({});
  }, [navigate]);

  useEffect(() => {
    let mounted = true;

    async function bootstrap() {
      if (!user) {
        setReady(true);
        return;
      }

      try {
        const [csrf, currentUser] = await Promise.all([fetchCsrfToken(), fetchCurrentUser()]);
        if (mounted) {
          const nextUser = currentUser?.user || user;
          setCsrfToken(csrf?.csrf_token || '');
          applyUser(nextUser);
        }
      } catch {
        if (mounted) {
          applyUser(null);
          clearCsrfToken();
        }
      } finally {
        if (mounted) {
          setReady(true);
        }
      }
    }

    bootstrap();

    return () => {
      mounted = false;
    };
  }, [applyUser, user]);

  const login = useCallback(async (username, password) => {
    const result = await loginApi({ username, password });
    const nextUser = result?.user || null;
    try {
      const csrf = await fetchCsrfToken();
      setCsrfToken(csrf?.csrf_token || '');
      applyUser(nextUser);
      return nextUser;
    } catch (err) {
      try {
        await logoutApi();
      } catch {
        // Best effort cleanup for sessions that cannot access the admin area.
      }
      clearCsrfToken();
      applyUser(null);
      throw err;
    }
  }, [applyUser]);

  const logout = useCallback(async () => {
    try {
      await logoutApi();
    } finally {
      clearCsrfToken();
      applyUser(null);
    }
  }, [applyUser]);

  const value = useMemo(
    () => ({
      user,
      ready,
      isAuthenticated: Boolean(user),
      login,
      logout,
      refreshCurrentUser,
    }),
    [login, logout, ready, refreshCurrentUser, user],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within AuthProvider');
  }
  return context;
}
