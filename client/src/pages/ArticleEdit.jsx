import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useI18n } from '../contexts/I18nContext';
import { categoryDisplayName } from '../i18n/displayNames';
import { createArticle, fetchArticle, fetchCategories, updateArticle, uploadImage } from '../utils/adminApi';
import { fromDateTimeInputValue, toDateTimeInputValue } from '../utils/format';

const initialForm = {
  title: '',
  content: '',
  cover_image: '',
  category_id: '',
  status: 'draft',
  published_at: '',
};

function inferReadTime(content) {
  const length = [...(content || '')].length;
  return Math.max(1, Math.ceil(length / 400));
}

export default function ArticleEdit() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { t } = useI18n();
  const isCreate = !id;
  const [form, setForm] = useState(initialForm);
  const [categories, setCategories] = useState([]);
  const [saving, setSaving] = useState(false);
  const [uploadingCover, setUploadingCover] = useState(false);
  const [coverMessage, setCoverMessage] = useState('');
  const [isPinned, setIsPinned] = useState(false);
  const [slug, setSlug] = useState('');

  useEffect(() => {
    let active = true;
    fetchCategories()
      .then((result) => {
        if (active) {
          setCategories(result?.list || []);
        }
      })
      .catch(() => {
        if (active) {
          setCategories([]);
        }
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (isCreate) {
      setForm(initialForm);
      setSlug('');
      setIsPinned(false);
      return undefined;
    }

    let active = true;
    fetchArticle(id)
      .then((article) => {
        if (!active) {
          return;
        }
        setSlug(article?.slug || '');
        setIsPinned(Boolean(article?.is_pinned));
        setForm({
          title: article?.title || '',
          content: article?.content || '',
          cover_image: article?.cover_image || '',
          category_id: article?.category_id ? String(article.category_id) : '',
          status: article?.status || 'draft',
          published_at: toDateTimeInputValue(article?.published_at),
        });
      })
      .catch(() => {
        if (active) {
          navigate('/posts', { replace: true });
        }
      });
    return () => {
      active = false;
    };
  }, [id, isCreate, navigate]);

  const preview = useMemo(() => {
    const category = categories.find((item) => String(item.id) === form.category_id);
    return {
      title: form.title || t('article.previewUntitled'),
      category: categoryDisplayName(t, category) || t('article.previewCategoryFallback'),
      readTime: inferReadTime(form.content),
      excerpt:
        form.content
          .replace(/[#>*`]/g, ' ')
          .replace(/\s+/g, ' ')
          .trim()
          .slice(0, 180) || t('article.previewExcerptFallback'),
    };
  }, [categories, form, t]);

  async function handleSubmit(event) {
    event.preventDefault();
    setSaving(true);
    try {
      const payload = {
        title: form.title,
        content: form.content,
        cover_image: form.cover_image,
        category_id: form.category_id ? Number(form.category_id) : null,
        status: form.status,
        is_pinned: isPinned,
        published_at: form.published_at ? fromDateTimeInputValue(form.published_at) : '',
      };
      const result = isCreate ? await createArticle(payload) : await updateArticle(id, payload);
      if (result?.id) {
        navigate(`/articles/${result.id}`, { replace: true });
      }
    } finally {
      setSaving(false);
    }
  }

  async function handleCoverUpload(event) {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }
    setUploadingCover(true);
    setCoverMessage('');
    try {
      const result = await uploadImage(file);
      if (result?.url) {
        setForm((prev) => ({ ...prev, cover_image: result.url }));
        setCoverMessage(t('article.coverUploadSuccess'));
      }
    } finally {
      setUploadingCover(false);
      event.target.value = '';
    }
  }

  return (
    <div className="admin-page">
      <section className="admin-page__header admin-page__header--split">
        <div>
          <h2>{isCreate ? t('article.createTitle') : t('article.editTitle')}</h2>
          <p>{t('article.pageDesc')}</p>
        </div>
        <div className="admin-page__header-meta">
          {!isCreate && slug ? <span>{t('article.slugPrefix', { slug })}</span> : null}
          <button type="button" className="admin-secondary-button" onClick={() => navigate('/posts')}>
            {t('common.backToPosts')}
          </button>
        </div>
      </section>

      <form className="admin-editor-layout" onSubmit={handleSubmit}>
        <article className="admin-panel">
          <div className="admin-panel__head admin-panel__head--stacked">
            <div>
              <h3>{t('article.body')}</h3>
              <p>{t('article.bodyDesc')}</p>
            </div>
          </div>

          <div className="admin-form">
            <label>
              <span>{t('article.title')}</span>
              <input
                value={form.title}
                onChange={(event) => setForm((prev) => ({ ...prev, title: event.target.value }))}
                placeholder={t('article.titlePlaceholder')}
                required
              />
            </label>
            <label>
              <span>{t('article.markdown')}</span>
              <textarea
                value={form.content}
                onChange={(event) => setForm((prev) => ({ ...prev, content: event.target.value }))}
                placeholder={t('article.markdownPlaceholder')}
                rows={22}
                required
              />
            </label>
          </div>
        </article>

        <aside className="admin-editor-sidebar">
          <article className="admin-panel">
            <div className="admin-panel__head admin-panel__head--stacked">
              <div>
                <h3>{t('article.publishing')}</h3>
                <p>{t('article.publishingDesc')}</p>
              </div>
            </div>

            <div className="admin-form">
              <label>
                <span>{t('article.category')}</span>
                <select
                  value={form.category_id}
                  onChange={(event) => setForm((prev) => ({ ...prev, category_id: event.target.value }))}
                >
                  <option value="">{t('common.uncategorized')}</option>
                  {categories.map((item) => (
                    <option key={item.id} value={item.id}>
                      {categoryDisplayName(t, item)}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>{t('article.status')}</span>
                <select
                  value={form.status}
                  onChange={(event) => setForm((prev) => ({ ...prev, status: event.target.value }))}
                >
                  <option value="draft">{t('common.statusDraft')}</option>
                  <option value="published">{t('common.statusPublished')}</option>
                </select>
              </label>
              <label>
                <span>{t('article.publishedAt')}</span>
                <input
                  type="datetime-local"
                  value={form.published_at}
                  onChange={(event) => setForm((prev) => ({ ...prev, published_at: event.target.value }))}
                />
              </label>
              <label className="admin-checkbox">
                <input type="checkbox" checked={isPinned} onChange={(event) => setIsPinned(event.target.checked)} />
                <span>{t('article.pinArticle')}</span>
              </label>
            </div>
          </article>

          <article className="admin-panel">
            <div className="admin-panel__head admin-panel__head--stacked">
              <div>
                <h3>{t('article.cover')}</h3>
                <p>{t('article.coverDesc')}</p>
              </div>
            </div>

            <div className="admin-form">
              <label>
                <span>{t('article.coverUrl')}</span>
                <input
                  value={form.cover_image}
                  onChange={(event) => setForm((prev) => ({ ...prev, cover_image: event.target.value }))}
                  placeholder={t('article.coverPlaceholder')}
                />
              </label>
              <label>
                <span>{t('article.coverUpload')}</span>
                <input
                  type="file"
                  accept="image/png,image/jpeg,image/gif,image/webp"
                  onChange={handleCoverUpload}
                  disabled={uploadingCover}
                />
              </label>
              <div className="admin-form__actions">
                <button
                  type="button"
                  className="admin-secondary-button"
                  onClick={() => setForm((prev) => ({ ...prev, cover_image: '' }))}
                  disabled={!form.cover_image || uploadingCover}
                >
                  {t('article.clearCover')}
                </button>
                {uploadingCover ? <span className="admin-upload-status">{t('article.coverUploading')}</span> : null}
              </div>
              {coverMessage ? <div className="admin-inline-banner">{coverMessage}</div> : null}

              <div className="admin-cover-preview">
                {form.cover_image ? <img src={form.cover_image} alt={t('article.coverPreviewAlt')} /> : <span>{t('common.noCoverSelected')}</span>}
              </div>
            </div>
          </article>

          <article className="admin-panel">
            <div className="admin-panel__head admin-panel__head--stacked">
              <div>
                <h3>{t('article.readerPreview')}</h3>
                <p>{t('article.readerPreviewDesc')}</p>
              </div>
            </div>

            <div className="admin-preview-card">
              <span className="admin-preview-card__tag">{preview.category}</span>
              <h4>{preview.title}</h4>
              <p>{preview.excerpt}</p>
              <div className="admin-preview-card__meta">
                <span>{t('article.readTime', { minutes: preview.readTime })}</span>
                <span>{form.status === 'published' ? t('common.statusPublished') : t('common.statusDraft')}</span>
              </div>
            </div>

            <div className="admin-form__actions">
              <button type="submit" className="admin-primary-button" disabled={saving}>
                {saving ? t('common.saving') : isCreate ? t('article.createArticle') : t('article.saveChanges')}
              </button>
            </div>
          </article>
        </aside>
      </form>
    </div>
  );
}
