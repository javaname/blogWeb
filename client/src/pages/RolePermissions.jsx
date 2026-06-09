import { useEffect, useState } from 'react';
import AdminIcon from '../components/AdminIcon';
import { useAuth } from '../contexts/AuthContext';
import { useI18n } from '../contexts/I18nContext';
import { fetchUsers, updateRolePermissions } from '../utils/adminApi';
import { showAdminToast } from '../utils/api';

function roleLabel(t, role) {
  return t(`users.roles.${role}`) || role;
}

function normalizeRolePermissionForm(roles) {
  return (roles || []).map((role) => ({
    key: role.key,
    permissions: [...(role.permissions || [])],
  }));
}

export default function RolePermissions() {
  const { refreshCurrentUser } = useAuth();
  const { t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [roles, setRoles] = useState([]);
  const [permissions, setPermissions] = useState([]);
  const [rolePermissionForm, setRolePermissionForm] = useState([]);
  const [message, setMessage] = useState('');

  async function loadRolePermissions() {
    setLoading(true);
    try {
      const payload = await fetchUsers();
      const nextRoles = payload?.roles || [];
      setRoles(nextRoles);
      setPermissions(payload?.permissions || []);
      setRolePermissionForm(normalizeRolePermissionForm(nextRoles));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadRolePermissions().catch(() => {
      setRoles([]);
      setPermissions([]);
      setRolePermissionForm([]);
    });
  }, []);

  function handleRolePermissionToggle(roleKey, permissionKey) {
    setRolePermissionForm((current) =>
      current.map((role) => {
        if (role.key !== roleKey) {
          return role;
        }
        const exists = role.permissions.includes(permissionKey);
        return {
          ...role,
          permissions: exists
            ? role.permissions.filter((permission) => permission !== permissionKey)
            : [...role.permissions, permissionKey],
        };
      }),
    );
  }

  async function handleRolePermissionsSubmit(event) {
    event.preventDefault();
    setSaving(true);
    setMessage('');
    try {
      const payload = await updateRolePermissions({ roles: rolePermissionForm });
      const nextRoles = payload?.roles || [];
      setRoles(nextRoles);
      setPermissions(payload?.permissions || permissions);
      setRolePermissionForm(normalizeRolePermissionForm(nextRoles));
      const successMessage = t('users.rolePermissionsSaved');
      setMessage(successMessage);
      showAdminToast('success', successMessage);
      await refreshCurrentUser();
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="admin-page" data-page="roles">
      <section className="admin-page__header admin-page__header--split">
        <div>
          <h2>{t('users.rolePermissionsTitle')}</h2>
          <p>{t('users.rolePermissionsDesc')}</p>
        </div>
      </section>

      {message ? <div className="admin-inline-banner is-success">{message}</div> : null}
      {loading ? <p className="admin-inline-state">{t('users.loading')}</p> : null}

      <section className="admin-two-column admin-role-permissions-page">
        <article className="admin-panel">
          <form className="admin-role-permissions" data-role-permissions-form onSubmit={handleRolePermissionsSubmit}>
            <div className="admin-panel__head admin-panel__head--stacked">
              <div>
                <h3>{t('users.rolePermissionMatrixTitle')}</h3>
                <p>{t('users.rolePermissionMatrixDesc')}</p>
              </div>
            </div>
            <div className="admin-role-permission-list">
              {rolePermissionForm.map((role) => (
                <div key={role.key} className="admin-role-permission-row">
                  <div className="admin-role-permission-row__head">
                    <strong>{roleLabel(t, role.key)}</strong>
                    <span>{t('users.rolePermissionCount', { count: role.permissions.length })}</span>
                  </div>
                  <div className="admin-role-permission-options">
                    {permissions.map((permission) => (
                      <label key={permission.key} className="admin-permission-check">
                        <input
                          type="checkbox"
                          checked={role.permissions.includes(permission.key)}
                          onChange={() => handleRolePermissionToggle(role.key, permission.key)}
                        />
                        <span>{permission.label || t(`users.permissionLabels.${permission.key}`)}</span>
                      </label>
                    ))}
                  </div>
                </div>
              ))}
            </div>
            <div className="admin-form__actions">
              <button type="submit" className="admin-primary-button" disabled={saving || loading}>
                {saving ? t('common.saving') : t('users.saveRolePermissions')}
              </button>
              <button
                type="button"
                className="admin-secondary-button"
                onClick={() => setRolePermissionForm(normalizeRolePermissionForm(roles))}
                disabled={saving}
              >
                {t('common.reset')}
              </button>
            </div>
          </form>
        </article>

        <aside className="admin-panel">
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
        </aside>
      </section>
    </div>
  );
}
