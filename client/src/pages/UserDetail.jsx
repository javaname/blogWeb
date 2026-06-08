import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';
import { categoryDisplayName, userDisplayName } from '../i18n/displayNames';
import { fetchUser, updateUser } from '../utils/adminApi';
import { showAdminToast } from '../utils/api';
import { formatDateTime } from '../utils/format';

const initialForm = {
  username: '',
  email: '',
  role: 'user',
};

function userInitials(t, user) {
  return (userDisplayName(t, user) || user?.username || user?.email || '?').slice(0, 2).toUpperCase();
}

function roleLabel(t, role) {
  return t(`users.roles.${role}`) || role;
}

function statusLabel(t, status) {
  return status === 'published' ? t('common.statusPublished') : t('common.statusDraft');
}

export default function UserDetail() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { locale, t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [user, setUser] = useState(null);
  const [roles, setRoles] = useState([]);
  const [permissions, setPermissions] = useState([]);
  const [recentArticles, setRecentArticles] = useState([]);
  const [form, setForm] = useState(initialForm);
  const [message, setMessage] = useState('');

  async function loadUser() {
    setLoading(true);
    try {
      const payload = await fetchUser(id);
      const nextUser = payload?.user || null;
      setUser(nextUser);
      setRoles(payload?.roles || []);
      setPermissions(payload?.permissions || []);
      setRecentArticles(payload?.recent_articles || []);
      setForm({
        username: nextUser?.username || '',
        email: nextUser?.email || '',
        role: nextUser?.role || 'user',
      });
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadUser().catch(() => {
      navigate('/users', { replace: true });
    });
  }, [id, navigate]);

  const permissionMap = useMemo(() => {
    return new Map((permissions || []).map((item) => [item.key, item]));
  }, [permissions]);

  const activeRole = roles.find((role) => role.key === form.role);
  const activePermissions = activeRole?.permissions || user?.permissions || [];

  async function handleSubmit(event) {
    event.preventDefault();
    setSaving(true);
    setMessage('');
    try {
      const payload = await updateUser(id, form);
      const nextUser = payload?.user || null;
      setUser(nextUser);
      setRoles(payload?.roles || roles);
      setPermissions(payload?.permissions || permissions);
      setRecentArticles(payload?.recent_articles || recentArticles);
      setForm({
        username: nextUser?.username || '',
        email: nextUser?.email || '',
        role: nextUser?.role || 'user',
      });
      const successMessage = t('users.detailSaved');
      setMessage(successMessage);
      showAdminToast('success', successMessage);
    } finally {
      setSaving(false);
    }
  }

  function resetForm() {
    setForm({
      username: user?.username || '',
      email: user?.email || '',
      role: user?.role || 'user',
    });
  }

  return (
    <div className="admin-page user-detail-page" data-page="user-detail">
      <section className="admin-page__header admin-page__header--split">
        <div>
          <h2>{t('users.detailTitle')}</h2>
          <p>{t('users.detailDesc')}</p>
        </div>
        <button type="button" className="admin-secondary-button" onClick={() => navigate('/users')}>
          {t('users.backToUsers')}
        </button>
      </section>

      {message ? <div className="admin-inline-banner is-success">{message}</div> : null}

      <section className="admin-two-column admin-user-detail-layout">
        <article className="admin-panel admin-user-detail-profile">
          <div className="admin-user-detail-profile__head">
            <span className="admin-user-detail-profile__avatar">{userInitials(t, user)}</span>
            <div>
              <h3>{user ? userDisplayName(t, user) : t('common.loading')}</h3>
              <p>{user?.email || t('users.noEmail')}</p>
            </div>
          </div>

          <div className="admin-user-detail-meta">
            <div>
              <span>{t('users.detailMeta.role')}</span>
              <strong>{roleLabel(t, user?.role)}</strong>
            </div>
            <div>
              <span>{t('users.detailMeta.articles')}</span>
              <strong>{user?.article_count ?? 0}</strong>
            </div>
            <div>
              <span>{t('users.detailMeta.created')}</span>
              <strong>{formatDateTime(user?.created_at, locale)}</strong>
            </div>
          </div>

          <form className="admin-form" data-user-detail-form onSubmit={handleSubmit}>
            <label>
              <span>{t('users.form.username')}</span>
              <input
                value={form.username}
                onChange={(event) => setForm((prev) => ({ ...prev, username: event.target.value }))}
                placeholder={t('users.form.usernamePlaceholder')}
                required
              />
            </label>
            <label>
              <span>{t('users.form.email')}</span>
              <input
                type="email"
                value={form.email}
                onChange={(event) => setForm((prev) => ({ ...prev, email: event.target.value }))}
                placeholder={t('users.form.emailPlaceholder')}
                required
              />
            </label>
            <label>
              <span>{t('users.form.role')}</span>
              <select value={form.role} onChange={(event) => setForm((prev) => ({ ...prev, role: event.target.value }))}>
                {roles.map((role) => (
                  <option key={role.key} value={role.key}>
                    {roleLabel(t, role.key)}
                  </option>
                ))}
              </select>
            </label>
            <div className="admin-form__actions">
              <button type="submit" className="admin-primary-button" disabled={saving || loading}>
                {saving ? t('common.saving') : t('users.saveDetail')}
              </button>
              <button type="button" className="admin-secondary-button" onClick={resetForm} disabled={!user || saving}>
                {t('common.reset')}
              </button>
            </div>
          </form>
        </article>

        <div className="admin-user-detail-stack">
          <article className="admin-panel">
            <div className="admin-panel__head admin-panel__head--stacked">
              <div>
                <h3>{t('users.detailPermissionsTitle')}</h3>
                <p>{t('users.detailPermissionsDesc')}</p>
              </div>
            </div>
            <div className="admin-permission-list">
              {activePermissions.map((permission) => {
                const definition = permissionMap.get(permission);
                return (
                  <div key={permission} className="admin-permission-list__row">
                    <AdminIcon name="visibility" />
                    <div>
                      <strong>{definition?.label || t(`users.permissionLabels.${permission}`)}</strong>
                      <p>{definition?.description || t(`users.permissions.${permission}.desc`)}</p>
                    </div>
                  </div>
                );
              })}
              {activePermissions.length === 0 ? <div className="admin-list-table__empty">{t('users.noPermissions')}</div> : null}
            </div>
          </article>

          <article className="admin-panel admin-related-articles">
            <div className="admin-panel__head admin-panel__head--stacked">
              <div>
                <h3>{t('users.relatedArticlesTitle')}</h3>
                <p>{t('users.relatedArticlesDesc')}</p>
              </div>
            </div>
            <div className="admin-list-table admin-list-table--related-articles">
              {recentArticles.map((article) => (
                <div key={article.id} className="admin-list-table__row admin-related-article-row">
                  <div>
                    <strong>{article.title}</strong>
                    <p>{article.slug}</p>
                  </div>
                  <span className={`admin-status-pill ${article.status === 'published' ? 'is-published' : 'is-draft'}`}>
                    {statusLabel(t, article.status)}
                  </span>
                  <span className="admin-category-pill">
                    {categoryDisplayName(t, article.category) || t('common.uncategorized')}
                  </span>
                  <span>{formatDateTime(article.published_at || article.updated_at, locale)}</span>
                  <button
                    type="button"
                    className="admin-icon-button"
                    onClick={() => navigate(`/articles/${article.id}`)}
                    aria-label={t('common.edit')}
                    title={t('common.edit')}
                  >
                    <AdminIcon name="edit" />
                  </button>
                </div>
              ))}
              {!loading && recentArticles.length === 0 ? (
                <div className="admin-list-table__empty">{t('users.noRelatedArticles')}</div>
              ) : null}
              {loading ? <div className="admin-list-table__empty">{t('users.loadingDetail')}</div> : null}
            </div>
          </article>
        </div>
      </section>
    </div>
  );
}
