import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';
import { userDisplayName } from '../i18n/displayNames';
import { createUser, deleteUser, fetchUsers, updateUserRole } from '../utils/adminApi';
import { showAdminToast } from '../utils/api';
import { formatDateTime } from '../utils/format';

const initialForm = {
  username: '',
  email: '',
  password: '',
  role: 'writer',
};

function articleCount(user) {
  return Number(user.article_count || 0);
}

function userHasArticles(user) {
  return articleCount(user) > 0;
}

function userInitials(t, user) {
  return (userDisplayName(t, user) || user.username || user.email || '?').slice(0, 2).toUpperCase();
}

function roleLabel(t, role) {
  return t(`users.roles.${role}`) || role;
}

function accountLine(t, user) {
  return t('users.accountLine', {
    username: user.username || '-',
    email: user.email || t('users.noEmail'),
  });
}

function articleCountLabel(t, user) {
  const count = articleCount(user);
  return count > 0 ? t('users.articleCount', { count }) : t('users.noArticles');
}

export default function Users() {
  const navigate = useNavigate();
  const { locale, t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [users, setUsers] = useState([]);
  const [roles, setRoles] = useState([]);
  const [permissions, setPermissions] = useState([]);
  const [form, setForm] = useState(initialForm);
  const [message, setMessage] = useState('');

  async function loadUsers() {
    setLoading(true);
    try {
      const payload = await fetchUsers();
      setUsers(payload?.list || []);
      setRoles(payload?.roles || []);
      setPermissions(payload?.permissions || []);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadUsers().catch(() => {
      setUsers([]);
      setRoles([]);
      setPermissions([]);
    });
  }, []);

  const stats = useMemo(() => {
    const adminCount = users.filter((user) => user.role === 'admin').length;
    const editorCount = users.filter((user) => user.role === 'editor').length;
    return {
      total: users.length,
      admins: adminCount,
      editors: editorCount,
      writers: users.filter((user) => user.role === 'writer').length,
    };
  }, [users]);

  async function handleCreate(event) {
    event.preventDefault();
    setSaving(true);
    setMessage('');
    try {
      await createUser(form);
      const successMessage = t('users.created');
      setForm(initialForm);
      setMessage(successMessage);
      showAdminToast('success', successMessage);
      await loadUsers();
    } finally {
      setSaving(false);
    }
  }

  async function handleRoleChange(id, role) {
    await updateUserRole(id, { role });
    const successMessage = t('users.roleUpdated');
    setMessage(successMessage);
    showAdminToast('success', successMessage);
    await loadUsers();
  }

  async function handleDelete(id) {
    await deleteUser(id);
    const successMessage = t('users.deleted');
    setMessage(successMessage);
    showAdminToast('success', successMessage);
    await loadUsers();
  }

  return (
    <div className="admin-page" data-page="users">
      <section className="admin-page__header admin-page__header--split">
        <div>
          <h2>{t('users.pageTitle')}</h2>
          <p>{t('users.pageDesc')}</p>
        </div>
      </section>

      {message ? <div className="admin-inline-banner is-success">{message}</div> : null}

      <section className="admin-stats-grid">
        {[
          ['total', stats.total],
          ['admins', stats.admins],
          ['editors', stats.editors],
          ['writers', stats.writers],
        ].map(([item, value]) => (
          <article key={item} className="admin-stat-card">
            <div className="admin-stat-card__top">
              <div className="admin-stat-card__icon tone-neutral">
                <AdminIcon name="group" />
              </div>
            </div>
            <p>{t(`users.stats.${item}`)}</p>
            <h3>{value}</h3>
          </article>
        ))}
      </section>

      <section className="admin-dashboard-grid">
        <article className="admin-panel">
          <div className="admin-panel__head">
            <div>
              <h3>{t('users.listTitle')}</h3>
              <p>{t('users.listDesc')}</p>
            </div>
          </div>

          <div className="admin-list-table admin-list-table--users">
            <div className="admin-list-table__head admin-user-grid">
              <span>{t('users.columns.name')}</span>
              <span>{t('users.columns.role')}</span>
              <span>{t('users.columns.permissions')}</span>
              <span>{t('users.columns.articles')}</span>
              <span className="align-right">{t('common.actions')}</span>
            </div>
            {users.map((user) => {
              const deleteBlocked = userHasArticles(user);
              const displayName = userDisplayName(t, user);
              const deleteLabel = deleteBlocked ? t('users.deleteBlockedByArticles') : t('common.delete');
              return (
                <div key={user.id} className="admin-list-table__row admin-user-grid">
                  <div className="admin-user-cell">
                    <span className="admin-user-cell__avatar">{userInitials(t, user)}</span>
                    <div className="admin-user-cell__body">
                      <strong className="admin-user-cell__display">{displayName}</strong>
                      <p className="admin-user-cell__account">{accountLine(t, user)}</p>
                      <p className="admin-user-cell__created">
                        {t('users.createdAt', { date: formatDateTime(user.created_at, locale) })}
                      </p>
                    </div>
                  </div>
                  <select
                    aria-label={t('users.changeRole')}
                    data-user-role-select
                    value={user.role}
                    onChange={(event) => handleRoleChange(user.id, event.target.value)}
                  >
                    {roles.map((role) => (
                      <option key={role.key} value={role.key}>
                        {roleLabel(t, role.key)}
                      </option>
                    ))}
                  </select>
                  <div className="admin-permission-pills">
                    {(user.permissions || []).map((permission) => (
                      <span key={permission}>{t(`users.permissionLabels.${permission}`)}</span>
                    ))}
                    {(user.permissions || []).length === 0 ? <span>{t('users.noPermissions')}</span> : null}
                  </div>
                  <div className={`admin-user-articles ${deleteBlocked ? 'is-linked' : 'is-clear'}`}>
                    <strong>{articleCountLabel(t, user)}</strong>
                    <span>{deleteBlocked ? t('users.articleDeleteBlocked') : t('users.articleDeleteAvailable')}</span>
                  </div>
                  <div className="admin-row-actions">
                    <button
                      type="button"
                      className="admin-icon-button"
                      data-user-edit
                      onClick={() => navigate(`/users/${user.id}`)}
                      aria-label={t('common.edit')}
                      title={t('common.edit')}
                    >
                      <AdminIcon name="edit" />
                    </button>
                    <button
                      type="button"
                      className="admin-icon-button is-danger"
                      data-user-delete
                      disabled={deleteBlocked}
                      onClick={() => handleDelete(user.id)}
                      aria-label={deleteLabel}
                      title={deleteLabel}
                    >
                      <AdminIcon name="delete" />
                    </button>
                  </div>
                </div>
              );
            })}
            {loading ? <div className="admin-list-table__empty">{t('users.loading')}</div> : null}
            {!loading && users.length === 0 ? <div className="admin-list-table__empty">{t('users.empty')}</div> : null}
          </div>
        </article>

        <aside className="admin-panel">
          <div className="admin-panel__head admin-panel__head--stacked">
            <div>
              <h3>{t('users.createTitle')}</h3>
              <p>{t('users.createDesc')}</p>
            </div>
          </div>
          <form className="admin-form" data-user-create-form onSubmit={handleCreate}>
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
              <span>{t('users.form.password')}</span>
              <input
                type="password"
                value={form.password}
                onChange={(event) => setForm((prev) => ({ ...prev, password: event.target.value }))}
                placeholder={t('users.form.passwordPlaceholder')}
                minLength={8}
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
              <button type="submit" className="admin-primary-button" disabled={saving}>
                {saving ? t('common.saving') : t('users.createUser')}
              </button>
              <button type="button" className="admin-secondary-button" onClick={() => setForm(initialForm)}>
                {t('common.reset')}
              </button>
            </div>
          </form>

          <div className="admin-user-permissions">
            <div className="admin-panel__head admin-panel__head--stacked">
              <div>
                <h3>{t('users.permissionsTitle')}</h3>
                <p>{t('users.permissionsDesc')}</p>
              </div>
            </div>
            <div className="admin-permission-list">
              {permissions.map((item) => (
                <div key={item.key} className="admin-permission-list__row">
                  <AdminIcon name="visibility" />
                  <div>
                    <strong>{item.label || t(`users.permissionLabels.${item.key}`)}</strong>
                    <p>{item.description || t(`users.permissions.${item.key}.desc`)}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </aside>
      </section>
    </div>
  );
}
