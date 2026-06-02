import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';

const users = [
  { key: 'owner', role: 'admin', status: 'active', articles: 42 },
  { key: 'editor', role: 'editor', status: 'active', articles: 18 },
  { key: 'writer', role: 'writer', status: 'invited', articles: 7 },
];

const permissions = ['publish', 'moderate', 'settings', 'mcp'];

export default function Users() {
  const { t } = useI18n();

  return (
    <div className="admin-page" data-page="users">
      <section className="admin-page__header admin-page__header--split">
        <div>
          <h2>{t('users.pageTitle')}</h2>
          <p>{t('users.pageDesc')}</p>
        </div>
        <button type="button" className="admin-primary-button">
          <AdminIcon name="person_add" />
          <span>{t('users.inviteUser')}</span>
        </button>
      </section>

      <section className="admin-stats-grid">
        {['total', 'admins', 'editors', 'pending'].map((item) => (
          <article key={item} className="admin-stat-card">
            <div className="admin-stat-card__top">
              <div className="admin-stat-card__icon tone-neutral">
                <AdminIcon name="group" />
              </div>
            </div>
            <p>{t(`users.stats.${item}`)}</p>
            <h3>{t(`users.statValues.${item}`)}</h3>
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
              <span>{t('users.columns.status')}</span>
              <span>{t('users.columns.articles')}</span>
            </div>
            {users.map((item) => (
              <div key={item.key} className="admin-list-table__row admin-user-grid">
                <div className="admin-user-cell">
                  <span>{t(`users.initials.${item.key}`)}</span>
                  <div>
                    <strong>{t(`users.names.${item.key}`)}</strong>
                    <p>{t(`users.emails.${item.key}`)}</p>
                  </div>
                </div>
                <span className="admin-category-pill">{t(`users.roles.${item.role}`)}</span>
                <span className={`admin-status-pill is-${item.status === 'active' ? 'approved' : 'pending'}`}>
                  {t(`users.statuses.${item.status}`)}
                </span>
                <span>{item.articles}</span>
              </div>
            ))}
          </div>
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
              <div key={item} className="admin-permission-list__row">
                <AdminIcon name="visibility" />
                <div>
                  <strong>{t(`users.permissions.${item}.label`)}</strong>
                  <p>{t(`users.permissions.${item}.desc`)}</p>
                </div>
              </div>
            ))}
          </div>
        </aside>
      </section>
    </div>
  );
}
