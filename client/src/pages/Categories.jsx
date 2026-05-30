import { useEffect, useMemo, useState } from 'react';
import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';
import { categoryDisplayName } from '../i18n/displayNames';
import { createCategory, deleteCategory, fetchCategories, sortCategories, updateCategory } from '../utils/adminApi';

const initialForm = { id: '', name: '', slug: '' };

export default function Categories() {
  const { t } = useI18n();
  const [categories, setCategories] = useState([]);
  const [form, setForm] = useState(initialForm);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');

  async function loadCategories() {
    const result = await fetchCategories();
    setCategories(result?.list || []);
  }

  useEffect(() => {
    loadCategories().catch(() => setCategories([]));
  }, []);

  const ordered = useMemo(
    () => [...categories].sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0)),
    [categories],
  );

  async function handleSubmit(event) {
    event.preventDefault();
    setSaving(true);
    try {
      if (form.id) {
        await updateCategory(form.id, { name: form.name, slug: form.slug });
        setMessage(t('categories.updated'));
      } else {
        await createCategory({ name: form.name, slug: form.slug });
        setMessage(t('categories.created'));
      }
      setForm(initialForm);
      await loadCategories();
    } finally {
      setSaving(false);
    }
  }

  async function moveCategory(index, direction) {
    const next = [...ordered];
    const target = index + direction;
    if (target < 0 || target >= next.length) {
      return;
    }
    [next[index], next[target]] = [next[target], next[index]];
    setCategories(next);
    await sortCategories(next.map((item) => item.id));
    setMessage(t('categories.orderUpdated'));
    await loadCategories();
  }

  async function handleDelete(id) {
    await deleteCategory(id);
    setMessage(t('categories.deleted'));
    if (form.id === String(id)) {
      setForm(initialForm);
    }
    await loadCategories();
  }

  return (
    <div className="admin-page">
      <section className="admin-page__header admin-page__header--split">
        <div>
          <h2>{t('categories.pageTitle')}</h2>
          <p>{t('categories.pageDesc')}</p>
        </div>
      </section>

      {message ? <div className="admin-inline-banner">{message}</div> : null}

      <section className="admin-two-column">
        <article className="admin-panel">
          <div className="admin-panel__head admin-panel__head--stacked">
            <div>
              <h3>{t('categories.formTitle')}</h3>
              <p>{t('categories.formDesc')}</p>
            </div>
          </div>

          <form className="admin-form" onSubmit={handleSubmit}>
            <label>
              <span>{t('categories.name')}</span>
              <input
                value={form.name}
                onChange={(event) => setForm((prev) => ({ ...prev, name: event.target.value }))}
                placeholder={t('categories.namePlaceholder')}
                required
              />
            </label>
            <label>
              <span>{t('categories.slug')}</span>
              <input
                value={form.slug}
                onChange={(event) => setForm((prev) => ({ ...prev, slug: event.target.value }))}
                placeholder={t('categories.slugPlaceholder')}
                required
              />
            </label>
            <div className="admin-form__actions">
              <button type="submit" className="admin-primary-button" disabled={saving}>
                {saving ? t('common.saving') : form.id ? t('categories.saveCategory') : t('categories.createCategory')}
              </button>
              <button type="button" className="admin-secondary-button" onClick={() => setForm(initialForm)}>
                {t('common.reset')}
              </button>
            </div>
          </form>
        </article>

        <article className="admin-panel">
          <div className="admin-panel__head admin-panel__head--stacked">
            <div>
              <h3>{t('categories.listTitle')}</h3>
              <p>{t('categories.listDesc')}</p>
            </div>
          </div>

          <div className="admin-list-table">
            <div className="admin-list-table__head admin-category-grid">
              <span>{t('categories.name')}</span>
              <span>{t('categories.articles')}</span>
              <span>{t('categories.order')}</span>
              <span className="align-right">{t('common.actions')}</span>
            </div>
            {ordered.map((item, index) => (
              <div key={item.id} className="admin-list-table__row admin-category-grid">
                <div>
                  <strong>{categoryDisplayName(t, item)}</strong>
                  <p>{item.slug}</p>
                </div>
                <span>{item.article_count || 0}</span>
                <div className="admin-inline-actions">
                  <button type="button" className="admin-secondary-button admin-secondary-button--small" onClick={() => moveCategory(index, -1)}>
                    {t('categories.up')}
                  </button>
                  <button type="button" className="admin-secondary-button admin-secondary-button--small" onClick={() => moveCategory(index, 1)}>
                    {t('categories.down')}
                  </button>
                </div>
                <div className="admin-row-actions">
                  <button type="button" className="admin-icon-button" onClick={() => setForm({ id: String(item.id), name: item.name, slug: item.slug })} aria-label={t('common.edit')}>
                    <AdminIcon name="edit" />
                  </button>
                  <button type="button" className="admin-icon-button is-danger" onClick={() => handleDelete(item.id)} aria-label={t('common.delete')}>
                    <AdminIcon name="delete" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </article>
      </section>
    </div>
  );
}
