import { Navigate, useLocation } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { useI18n } from '../contexts/I18nContext';

export default function ProtectedRoute({ children }) {
  const { ready, isAuthenticated } = useAuth();
  const { t } = useI18n();
  const location = useLocation();

  if (!ready) {
    return (
      <div className="admin-loading-screen">
        <div className="admin-loading-screen__spinner" />
        <span>{t('protected.checkingSession')}</span>
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace state={{ from: location }} />;
  }

  return children;
}
