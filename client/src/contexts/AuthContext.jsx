import { createContext, useContext, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { fetchCsrfToken, login as loginApi, logout as logoutApi } from '../utils/adminApi';
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
        const result = await fetchCsrfToken();
        if (mounted) {
          setCsrfToken(result?.csrf_token || '');
        }
      } catch {
        if (mounted) {
          persistUser(null);
          setUser(null);
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
  }, [user]);

  async function login(username, password) {
    const result = await loginApi({ username, password });
    const nextUser = result?.user || null;
    try {
      const csrf = await fetchCsrfToken();
      setCsrfToken(csrf?.csrf_token || '');
      persistUser(nextUser);
      setUser(nextUser);
      return nextUser;
    } catch (err) {
      try {
        await logoutApi();
      } catch {
        // Best effort cleanup for sessions that cannot access the admin area.
      }
      clearCsrfToken();
      persistUser(null);
      setUser(null);
      throw err;
    }
  }

  async function logout() {
    try {
      await logoutApi();
    } finally {
      clearCsrfToken();
      persistUser(null);
      setUser(null);
    }
  }

  const value = useMemo(
    () => ({
      user,
      ready,
      isAuthenticated: Boolean(user),
      login,
      logout,
    }),
    [ready, user],
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
