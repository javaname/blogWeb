import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';

const metrics = [
  { key: 'views', value: '48.2K', icon: 'visibility', tone: 'primary' },
  { key: 'readTime', value: '6m12s', icon: 'article', tone: 'tertiary' },
  { key: 'returning', value: '41%', icon: 'group', tone: 'neutral' },
  { key: 'conversion', value: '8.7%', icon: 'trending_up', tone: 'primary-soft' },
];

const topContent = ['rust', 'design', 'workflow', 'systems'];
const sources = ['direct', 'search', 'social', 'newsletter'];

export default function Analytics() {
  const { t } = useI18n();

  return (
    <div className="admin-page" data-page="analytics">
      <section className="admin-page__header">
        <h2>{t('analytics.pageTitle')}</h2>
        <p>{t('analytics.pageDesc')}</p>
      </section>

      <section className="admin-stats-grid">
        {metrics.map((item) => (
          <article key={item.key} className="admin-stat-card">
            <div className="admin-stat-card__top">
              <div className={`admin-stat-card__icon tone-${item.tone}`}>
                <AdminIcon name={item.icon} />
              </div>
              <span className="admin-stat-card__change">
                {t(`analytics.changes.${item.key}`)}
                <AdminIcon name="trending_up" />
              </span>
            </div>
            <p>{t(`analytics.metrics.${item.key}`)}</p>
            <h3>{item.value}</h3>
          </article>
        ))}
      </section>

      <section className="admin-dashboard-grid">
        <article className="admin-panel admin-chart-panel">
          <div className="admin-panel__head">
            <div>
              <h3>{t('analytics.trendTitle')}</h3>
              <p>{t('analytics.trendDesc')}</p>
            </div>
            <span className="admin-filter-pill">{t('analytics.last30Days')}</span>
          </div>
          <div className="admin-chart admin-chart--compact">
            <div className="admin-chart__bars">
              {[44, 68, 52, 96, 72, 118, 86, 132, 104, 148].map((height, index) => (
                <span key={index} style={{ height }} />
              ))}
            </div>
          </div>
        </article>

        <aside className="admin-panel">
          <div className="admin-panel__head admin-panel__head--stacked">
            <div>
              <h3>{t('analytics.sourcesTitle')}</h3>
              <p>{t('analytics.sourcesDesc')}</p>
            </div>
          </div>
          <div className="admin-settings-list">
            {sources.map((item) => (
              <div key={item} className="admin-settings-list__row">
                <span>{t(`analytics.sources.${item}.label`)}</span>
                <strong>{t(`analytics.sources.${item}.value`)}</strong>
              </div>
            ))}
          </div>
        </aside>
      </section>

      <section className="admin-panel">
        <div className="admin-panel__head">
          <div>
            <h3>{t('analytics.contentTitle')}</h3>
            <p>{t('analytics.contentDesc')}</p>
          </div>
        </div>
        <div className="admin-list-table">
          {topContent.map((item) => (
            <div key={item} className="admin-list-table__row admin-analytics-row">
              <strong>{t(`analytics.content.${item}.title`)}</strong>
              <span>{t(`analytics.content.${item}.category`)}</span>
              <span>{t(`analytics.content.${item}.views`)}</span>
              <span>{t(`analytics.content.${item}.rate`)}</span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
