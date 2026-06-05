import { useMemo, useState } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { useI18n } from '../contexts/I18nContext';
import { userDisplayName } from '../i18n/displayNames';
import AdminIcon from './AdminIcon';
import LanguageSwitcher from './LanguageSwitcher';
import ThemeSwitcher from './ThemeSwitcher';
import useAdminRouteMotion from '../hooks/useAdminRouteMotion';

const navItems = [
  { key: '/dashboard', labelKey: 'shell.navDashboard', icon: 'dashboard' },
  { key: '/posts', labelKey: 'shell.navPosts', icon: 'article' },
  { key: '/categories', labelKey: 'shell.navCategories', icon: 'category' },
  { key: '/comments', labelKey: 'shell.navComments', icon: 'comment' },
  { key: '/media', labelKey: 'shell.navMedia', icon: 'image' },
  { key: '/users', labelKey: 'shell.navUsers', icon: 'group' },
  { key: '/analytics', labelKey: 'shell.navAnalytics', icon: 'trending_up' },
  { key: '/settings', labelKey: 'shell.navSettings', icon: 'settings' },
];

function isActive(pathname, itemKey) {
  if (itemKey === '/posts') {
    return pathname === '/posts' || pathname.startsWith('/articles/');
  }
  return pathname === itemKey;
}

export default function AppShell() {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { user, logout } = useAuth();
  const { t } = useI18n();
  const [openPanel, setOpenPanel] = useState('');
  useAdminRouteMotion(pathname);

  const adminActivity = useMemo(
    () => [
      {
        icon: 'publish',
        tone: 'primary',
        title: t('shell.activityPostPublished'),
        description: t('shell.activityPostPublishedDesc'),
        time: t('shell.timeTwoHoursAgo'),
      },
      {
        icon: 'add_comment',
        tone: 'tertiary',
        title: t('shell.activityNewComment'),
        description: t('shell.activityNewCommentDesc'),
        time: t('shell.timeFiveHoursAgo'),
      },
      {
        icon: 'person_add',
        tone: 'neutral',
        title: t('shell.activityNewFollower'),
        description: t('shell.activityNewFollowerDesc'),
        time: t('shell.timeYesterday'),
      },
      {
        icon: 'update',
        tone: 'primary',
        title: t('shell.activityPostUpdated'),
        description: t('shell.activityPostUpdatedDesc'),
        time: t('shell.timeTwoDaysAgo'),
      },
    ],
    [t],
  );

  async function handleLogout() {
    await logout();
    navigate('/login', { replace: true });
  }

  const notifications = useMemo(
    () => [
      [t('shell.notificationCommentTitle'), t('shell.notificationCommentDesc')],
      [t('shell.notificationDraftTitle'), t('shell.notificationDraftDesc')],
      [t('shell.notificationBackupTitle'), t('shell.notificationBackupDesc')],
    ],
    [t],
  );

  const helpItems = useMemo(
    () => [
      [t('shell.helpWriteTitle'), t('shell.helpWriteDesc')],
      [t('shell.helpModerateTitle'), t('shell.helpModerateDesc')],
      [t('shell.helpSettingsTitle'), t('shell.helpSettingsDesc')],
    ],
    [t],
  );

  function togglePanel(panel) {
    setOpenPanel((current) => (current === panel ? '' : panel));
  }

  return (
    <div className="admin-shell">
      <aside className="admin-sidebar">
        <div className="admin-sidebar__brand">
          <h1>{t('shell.brandTitle')}</h1>
          <p>{t('shell.brandSub')}</p>
        </div>

        <nav className="admin-nav">
          {navItems.map((item) => (
            <button
              key={item.key}
              type="button"
              className={`admin-nav__item ${isActive(pathname, item.key) ? 'is-active' : ''}`}
              onClick={() => navigate(item.key)}
            >
              <AdminIcon name={item.icon} />
              <span>{t(item.labelKey)}</span>
            </button>
          ))}
        </nav>

        <div className="admin-sidebar__footer">
          <button
            type="button"
            className="admin-primary-button admin-primary-button--full"
            onClick={() => navigate('/articles/new')}
          >
            <AdminIcon name="add" />
            <span>{t('common.newPost')}</span>
          </button>

          <div className="admin-usercard">
            <img
              alt={t('shell.adminAvatar')}
              src="https://lh3.googleusercontent.com/aida-public/AB6AXuDE0ev-JB33hUlLQkaVuyMf7_37CN0aUjNFznTFc_8Fe1vq5YW2CgcRZ_olG3bWCTIHWgPJzGZ8wilwB1ZtkpzNOsP0H7feDbBPK5WykNPQfXNXt5VhkfGX67z4EGUhndyicLImn1Yk2TTkYIO-_DEJag3nMAUGnmGZQVnTOJ5MW73XPM5rJq7KnTlwVS4g1dDW7MbiCjEpdiiE1yGIgHRlesapsdQ1_f2jeTSY_d9c3dMvT2Ir2eHEyPvzPwNpF2gGoJVnbJBP0A0"
            />
            <div>
              <p>{userDisplayName(t, user) || t('shell.fallbackUserName')}</p>
              <span>{t('shell.superAdmin')}</span>
            </div>
          </div>
        </div>
      </aside>

      <div className="admin-main">
        <header className="admin-topbar">
          <div className="admin-topbar__actions">
            <LanguageSwitcher compact />
            <ThemeSwitcher />
            <button
              type="button"
              className="admin-icon-button admin-icon-button--alert"
              aria-label={t('shell.notifications')}
              onClick={() => togglePanel('notifications')}
            >
              <AdminIcon name="notifications" />
            </button>
            <button type="button" className="admin-icon-button" aria-label={t('shell.help')} onClick={() => togglePanel('help')}>
              <AdminIcon name="help" />
            </button>
            <div className="admin-topbar__divider" />
            <button type="button" className="admin-avatar-button" onClick={handleLogout} aria-label={t('common.logout')}>
              <img
                alt={t('shell.userProfile')}
                src="https://lh3.googleusercontent.com/aida-public/AB6AXuAdRurO9IgRv7Ok_taeu4z98Ov-PonJTaeaQ5GKA0sUSPePG_RNp9K9R76-JWZDeeETPCUX1WQMvn4-7oPnTFd7gRF9smqICKTS3YMKtbgsk1j2i4uD8HPMId-ngjFPRNyBj78-FYRqINfsOGmB9RrW04ka19m-FnU9-P5iOy81t5O908z6ZaU-e6dJcvuzWYPW6jPLtOEfb_OPF7VV2Ns7Jzmqd3fp-DL9Y092ntbzktQYA9HuCOJKy8jLr0ZRq9tgR70yWNUL1XQ"
              />
            </button>
          </div>
          {openPanel === 'notifications' ? (
            <section className="admin-topbar-panel">
              <div className="admin-topbar-panel__head">
                <h3>{t('shell.notifications')}</h3>
                <button type="button" className="admin-text-action" onClick={() => setOpenPanel('')}>
                  {t('shell.closePanel')}
                </button>
              </div>
              <div className="admin-topbar-panel__list">
                {notifications.map(([title, description]) => (
                  <article key={title} className="admin-topbar-panel__item">
                    <strong>{title}</strong>
                    <p>{description}</p>
                  </article>
                ))}
              </div>
            </section>
          ) : null}
          {openPanel === 'help' ? (
            <section className="admin-topbar-panel">
              <div className="admin-topbar-panel__head">
                <h3>{t('shell.help')}</h3>
                <button type="button" className="admin-text-action" onClick={() => setOpenPanel('')}>
                  {t('shell.closePanel')}
                </button>
              </div>
              <div className="admin-topbar-panel__list">
                {helpItems.map(([title, description]) => (
                  <article key={title} className="admin-topbar-panel__item">
                    <strong>{title}</strong>
                    <p>{description}</p>
                  </article>
                ))}
              </div>
            </section>
          ) : null}
        </header>

        <main className="admin-canvas">
          <Outlet context={{ adminActivity }} />
        </main>
      </div>
    </div>
  );
}
