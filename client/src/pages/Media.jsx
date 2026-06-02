import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';

const mediaItems = [
  { key: 'hero', type: 'image', sizeKey: 'media.sizeLarge', date: '2026-06-01', icon: 'image' },
  { key: 'cover', type: 'image', sizeKey: 'media.sizeMedium', date: '2026-05-30', icon: 'image' },
  { key: 'chart', type: 'asset', sizeKey: 'media.sizeSmall', date: '2026-05-28', icon: 'visibility' },
  { key: 'author', type: 'avatar', sizeKey: 'media.sizeTiny', date: '2026-05-27', icon: 'group' },
];

const stats = [
  { key: 'files', icon: 'image', value: '128' },
  { key: 'storage', icon: 'dashboard', value: '842MB' },
  { key: 'used', icon: 'article', value: '74%' },
  { key: 'pending', icon: 'publish', value: '6' },
];

export default function Media() {
  const { t } = useI18n();

  return (
    <div className="admin-page" data-page="media">
      <section className="admin-page__header admin-page__header--split">
        <div>
          <h2>{t('media.pageTitle')}</h2>
          <p>{t('media.pageDesc')}</p>
        </div>
        <button type="button" className="admin-primary-button">
          <AdminIcon name="add" />
          <span>{t('media.uploadAsset')}</span>
        </button>
      </section>

      <section className="admin-stats-grid">
        {stats.map((item) => (
          <article key={item.key} className="admin-stat-card">
            <div className="admin-stat-card__top">
              <div className="admin-stat-card__icon tone-primary">
                <AdminIcon name={item.icon} />
              </div>
              <span className="admin-stat-card__change">{t('media.live')}</span>
            </div>
            <p>{t(`media.stats.${item.key}`)}</p>
            <h3>{item.value}</h3>
          </article>
        ))}
      </section>

      <section className="admin-dashboard-grid">
        <article className="admin-panel">
          <div className="admin-panel__head">
            <div>
              <h3>{t('media.libraryTitle')}</h3>
              <p>{t('media.libraryDesc')}</p>
            </div>
            <span className="admin-filter-pill">
              {t('media.allTypes')}
              <AdminIcon name="expand_more" />
            </span>
          </div>
          <div className="admin-media-grid">
            {mediaItems.map((item) => (
              <article key={item.key} className="admin-media-card">
                <div className="admin-media-card__preview">
                  <AdminIcon name={item.icon} />
                </div>
                <div>
                  <strong>{t(`media.items.${item.key}`)}</strong>
                  <p>{t(`media.types.${item.type}`)}</p>
                </div>
                <span>{t(item.sizeKey)}</span>
                <small>{item.date}</small>
              </article>
            ))}
          </div>
        </article>

        <aside className="admin-panel">
          <div className="admin-panel__head admin-panel__head--stacked">
            <div>
              <h3>{t('media.rulesTitle')}</h3>
              <p>{t('media.rulesDesc')}</p>
            </div>
          </div>
          <div className="admin-settings-list">
            {['formats', 'maxSize', 'path', 'altText'].map((item) => (
              <div key={item} className="admin-settings-list__row">
                <span>{t(`media.rules.${item}.label`)}</span>
                <strong>{t(`media.rules.${item}.value`)}</strong>
              </div>
            ))}
          </div>
        </aside>
      </section>
    </div>
  );
}
