import { useEffect, useMemo, useState } from 'react';
import { useI18n } from '../contexts/I18nContext';
import { fetchSettings, updateSettings } from '../utils/adminApi';

const emptySite = {
  title: '',
  description: '',
  base_url: '',
};

export default function Settings() {
  const { t } = useI18n();
  const [settings, setSettings] = useState(null);
  const [siteForm, setSiteForm] = useState(emptySite);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');

  useEffect(() => {
    let active = true;
    fetchSettings()
      .then((result) => {
        if (!active) {
          return;
        }
        setSettings(result || null);
        setSiteForm({
          title: result?.site?.title || '',
          description: result?.site?.description || '',
          base_url: result?.site?.base_url || '',
        });
      })
      .catch(() => {
        if (active) {
          setSettings(null);
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

  const settingsGroups = useMemo(
    () => [
      {
        title: t('settings.publishingDefaults'),
        description: t('settings.publishingDefaultsDesc'),
        rows: [
          [t('settings.defaultAuthor'), settings?.publishing?.default_author || t('common.admin')],
          [t('settings.scheduledPublishing'), settings?.publishing?.scheduled_publishing ? t('settings.valueEnabled') : t('settings.valueManual')],
          [t('settings.pinnedStories'), settings?.publishing?.pinned_stories || t('settings.valueManual')],
        ],
      },
      {
        title: t('settings.moderationPolicy'),
        description: t('settings.moderationPolicyDesc'),
        rows: [
          [t('settings.commentReview'), t('settings.valueAutoAndManual')],
          [t('settings.sensitiveRules'), t('settings.valueEnabled')],
          [t('settings.rejectionReason'), t('settings.valueRequired')],
        ],
      },
      {
        title: t('settings.systemStatus'),
        description: t('settings.systemStatusDesc'),
        rows: [
          [t('settings.apiProtection'), t('settings.valueCsrfEnabled')],
          [t('settings.uploadPolicy'), `${settings?.upload?.allowed_types?.length || 0} ${t('settings.valueImageTypes')}`],
          [t('settings.readerSignals'), t('settings.valueAnonymous')],
        ],
      },
      {
        title: t('settings.integrationStatus'),
        description: t('settings.integrationStatusDesc'),
        rows: [
          [t('settings.mcpHttp'), settings?.mcp?.http_enabled ? t('settings.valueEnabled') : t('settings.valueManual')],
          [t('settings.mcpOrigin'), settings?.mcp?.require_origin_check ? t('settings.valueEnabled') : t('settings.valueManual')],
          [t('settings.uploadSize'), `${Math.round((settings?.upload?.max_size || 0) / 1024 / 1024)}${t('settings.megabytes')}`],
        ],
      },
    ],
    [settings, t],
  );

  function updateField(field, value) {
    setSiteForm((current) => ({ ...current, [field]: value }));
  }

  async function handleSubmit(event) {
    event.preventDefault();
    setSaving(true);
    setMessage('');
    try {
      const nextSettings = await updateSettings({ site: siteForm });
      setSettings(nextSettings || null);
      setSiteForm({
        title: nextSettings?.site?.title || '',
        description: nextSettings?.site?.description || '',
        base_url: nextSettings?.site?.base_url || '',
      });
      setMessage(t('settings.savedMessage'));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="admin-page">
      <section className="admin-page__header">
        <h2>{t('settings.pageTitle')}</h2>
        <p>{t('settings.pageDesc')}</p>
      </section>

      {loading ? <p className="admin-inline-state">{t('settings.loading')}</p> : null}

      <section className="admin-panel">
        <div className="admin-panel__head admin-panel__head--stacked">
          <div>
            <h3>{t('settings.editorialIdentity')}</h3>
            <p>{t('settings.editorialIdentityDesc')}</p>
          </div>
        </div>
        <form className="admin-settings-form" onSubmit={handleSubmit}>
          <label>
            <span>{t('settings.siteTitle')}</span>
            <input value={siteForm.title} onChange={(event) => updateField('title', event.target.value)} maxLength={80} />
          </label>
          <label>
            <span>{t('settings.siteDescription')}</span>
            <textarea value={siteForm.description} onChange={(event) => updateField('description', event.target.value)} maxLength={200} rows={3} />
          </label>
          <label>
            <span>{t('settings.baseUrl')}</span>
            <input value={siteForm.base_url} onChange={(event) => updateField('base_url', event.target.value)} placeholder={t('settings.baseUrlPlaceholder')} />
          </label>
          <div className="admin-form__actions">
            <button type="submit" className="admin-primary-button" disabled={saving}>
              {saving ? t('common.saving') : t('settings.saveSite')}
            </button>
            {message ? <span className="admin-form-message">{message}</span> : null}
          </div>
        </form>
      </section>

      <section className="admin-settings-grid">
        {settingsGroups.map((group) => (
          <article key={group.title} className="admin-panel">
            <div className="admin-panel__head admin-panel__head--stacked">
              <div>
                <h3>{group.title}</h3>
                <p>{group.description}</p>
              </div>
            </div>
            <div className="admin-settings-list">
              {group.rows.map(([label, value]) => (
                <div key={label} className="admin-settings-list__row">
                  <span>{label}</span>
                  <strong>{value}</strong>
                </div>
              ))}
            </div>
          </article>
        ))}
      </section>

      <section className="admin-panel">
        <div className="admin-panel__head admin-panel__head--stacked">
          <div>
            <h3>{t('settings.operationalNotes')}</h3>
            <p>{t('settings.operationalNotesDesc')}</p>
          </div>
        </div>
        <div className="admin-note-grid">
          <article>
            <strong>{t('settings.noteDraftsTitle')}</strong>
            <p>{t('settings.noteDraftsDesc')}</p>
          </article>
          <article>
            <strong>{t('settings.noteUploadsTitle')}</strong>
            <p>{t('settings.noteUploadsDesc')}</p>
          </article>
          <article>
            <strong>{t('settings.noteFrontendTitle')}</strong>
            <p>{t('settings.noteFrontendDesc')}</p>
          </article>
        </div>
      </section>
    </div>
  );
}
