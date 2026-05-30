import { useEffect, useMemo, useState } from 'react';
import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';
import { fetchDashboard } from '../utils/adminApi';

const statCards = [
  { icon: 'article', tone: 'primary', trend: 'trending_up', labelKey: 'dashboard.statsTotalPosts', statKey: 'total_articles' },
  { icon: 'visibility', tone: 'tertiary', trend: 'trending_up', labelKey: 'dashboard.statsMonthlyViews', statKey: 'monthly_views' },
  { icon: 'chat_bubble', tone: 'neutral', trend: 'trending_up', labelKey: 'dashboard.statsNewComments', statKey: 'total_comments' },
  { icon: 'group', tone: 'primary-soft', trend: 'trending_up', labelKey: 'dashboard.statsFollowers', statKey: 'followers' },
];

function formatCompact(value) {
  const number = Number(value || 0);
  if (number < 1000) {
    return String(number);
  }
  if (number < 10000) {
    return `${(number / 1000).toFixed(1)}K`;
  }
  return `${Math.round(number / 1000)}K`;
}

function formatTrendDate(value) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value || '';
  }
  return `${date.getMonth() + 1}/${date.getDate()}`;
}

function activityTime(value) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return '';
  }
  return date.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

export default function Dashboard() {
  const { t } = useI18n();
  const [dashboard, setDashboard] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;
    fetchDashboard()
      .then((result) => {
        if (active) {
          setDashboard(result || null);
        }
      })
      .catch(() => {
        if (active) {
          setDashboard(null);
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, []);

  const stats = dashboard?.stats || {};
  const trend = dashboard?.views_trend || [];
  const activity = dashboard?.activity || [];

  const chartHeights = useMemo(() => {
    const values = trend.map((point) => Number(point.views || 0));
    const max = Math.max(...values, 1);
    return values.slice(-10).map((value) => Math.max(28, Math.round((value / max) * 120)));
  }, [trend]);

  const chartLabels = useMemo(() => {
    if (!trend.length) {
      return [];
    }
    return trend
      .filter((_, index) => index % Math.max(1, Math.floor(trend.length / 5)) === 0)
      .slice(0, 5)
      .map((point) => formatTrendDate(point.date));
  }, [trend]);

  return (
    <div className="admin-page">
      <section className="admin-page__header">
        <h2>{t('dashboard.pageTitle')}</h2>
        <p>{t('dashboard.pageDesc')}</p>
      </section>

      {loading ? <p className="admin-inline-state">{t('dashboard.loading')}</p> : null}

      <section className="admin-stats-grid">
        {statCards.map((card) => (
          <article key={card.labelKey} className="admin-stat-card">
            <div className="admin-stat-card__top">
              <div className={`admin-stat-card__icon tone-${card.tone}`}>
                <AdminIcon name={card.icon} />
              </div>
              <span className="admin-stat-card__change">
                {t('dashboard.liveData')}
                <AdminIcon name={card.trend} />
              </span>
            </div>
            <p>{t(card.labelKey)}</p>
            <h3>{formatCompact(stats[card.statKey])}</h3>
          </article>
        ))}
      </section>

      <section className="admin-dashboard-grid">
        <article className="admin-panel admin-chart-panel">
          <div className="admin-panel__head">
            <div>
              <h3>{t('dashboard.viewsChart')}</h3>
              <p>{t('dashboard.last30DaysPerformance')}</p>
            </div>
            <span className="admin-filter-pill">
              {t('dashboard.last30Days')}
              <AdminIcon name="expand_more" />
            </span>
          </div>

          <div className="admin-chart">
            <div className="admin-chart__bars">
              {(chartHeights.length ? chartHeights : [28, 28, 28, 28, 28]).map((height, index) => (
                <span key={index} style={{ height }} />
              ))}
            </div>
            <svg viewBox="0 0 1000 260" preserveAspectRatio="none" aria-hidden="true">
              <defs>
                <linearGradient id="dashboard-gradient" x1="0%" x2="0%" y1="0%" y2="100%">
                  <stop offset="0%" stopColor="#0058be" stopOpacity="0.22" />
                  <stop offset="100%" stopColor="#0058be" stopOpacity="0" />
                </linearGradient>
              </defs>
              <path
                d="M0 190 Q 80 170, 160 180 T 320 110 T 500 176 T 680 72 T 860 160 T 1000 150"
                fill="none"
                stroke="#0058be"
                strokeWidth="4"
                strokeLinecap="round"
              />
              <path
                d="M0 190 Q 80 170, 160 180 T 320 110 T 500 176 T 680 72 T 860 160 T 1000 150 V 260 H 0 Z"
                fill="url(#dashboard-gradient)"
              />
            </svg>
            <div className="admin-chart__labels">
              {(chartLabels.length ? chartLabels : [t('dashboard.noTrend')]).map((label) => (
                <span key={label}>{label}</span>
              ))}
            </div>
          </div>
        </article>

        <article className="admin-panel admin-activity-panel">
          <h3>{t('dashboard.recentActivity')}</h3>
          <div className="admin-activity-list">
            {(activity.length ? activity : []).map((item) => (
              <div key={`${item.title}-${item.created_at}`} className="admin-activity-item">
                <div className={`admin-activity-item__icon tone-${item.tone || 'neutral'}`}>
                  <AdminIcon name={item.icon || 'article'} />
                </div>
                <div>
                  <p className="admin-activity-item__title">{item.title}</p>
                  <p className="admin-activity-item__description">{item.description}</p>
                  <span className="admin-activity-item__time">{activityTime(item.created_at)}</span>
                </div>
              </div>
            ))}
            {!activity.length && !loading ? <p className="admin-inline-state">{t('dashboard.emptyActivity')}</p> : null}
          </div>
        </article>
      </section>

      <footer className="admin-footer">
        <div>
          <h3>{t('site.title')}</h3>
          <p>{t('dashboard.footerCopyright')}</p>
        </div>
        <nav>
          <a href="#privacy">{t('dashboard.privacyPolicy')}</a>
          <a href="#terms">{t('dashboard.termsOfService')}</a>
          <a href="#rss">{t('dashboard.rssFeed')}</a>
          <a href="#sitemap">{t('dashboard.sitemap')}</a>
        </nav>
      </footer>
    </div>
  );
}
