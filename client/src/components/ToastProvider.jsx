import { useCallback, useEffect, useRef, useState } from 'react';
import AdminIcon from './AdminIcon';
import { useI18n } from '../contexts/I18nContext';

const TOAST_TTL_MS = 4200;
const MAX_TOASTS = 4;

function nextToastId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export default function ToastProvider({ children }) {
  const { t } = useI18n();
  const [toasts, setToasts] = useState([]);
  const timers = useRef(new Map());

  const dismiss = useCallback((id) => {
    const timer = timers.current.get(id);
    if (timer) {
      window.clearTimeout(timer);
      timers.current.delete(id);
    }
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const pushToast = useCallback(
    (type, content) => {
      const message = String(content || '').trim();
      if (!message) {
        return;
      }
      const id = nextToastId();
      setToasts((current) => [{ id, type, content: message }, ...current].slice(0, MAX_TOASTS));
      const timer = window.setTimeout(() => dismiss(id), TOAST_TTL_MS);
      timers.current.set(id, timer);
    },
    [dismiss],
  );

  useEffect(() => {
    const toastApi = {
      success: (content) => pushToast('success', content),
      error: (content) => pushToast('error', content),
      info: (content) => pushToast('info', content),
    };
    window.__BLOG_ADMIN_TOAST__ = toastApi;

    return () => {
      if (window.__BLOG_ADMIN_TOAST__ === toastApi) {
        delete window.__BLOG_ADMIN_TOAST__;
      }
      timers.current.forEach((timer) => window.clearTimeout(timer));
      timers.current.clear();
    };
  }, [pushToast]);

  return (
    <>
      {children}
      <div className="admin-toast-viewport" aria-live="polite" aria-relevant="additions">
        {toasts.map((toast) => (
          <div key={toast.id} className={`admin-toast is-${toast.type}`} role="status">
            <AdminIcon name={toast.type === 'error' ? 'notifications' : 'visibility'} />
            <p>{toast.content}</p>
            <button type="button" onClick={() => dismiss(toast.id)} aria-label={t('shell.closePanel')}>
              <AdminIcon name="close" />
            </button>
          </div>
        ))}
      </div>
    </>
  );
}
