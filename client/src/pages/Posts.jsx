import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';
import { categoryDisplayName } from '../i18n/displayNames';
import { deleteArticle, fetchArticles, fetchCategories } from '../utils/adminApi';
import { formatDateTime } from '../utils/format';

export default function Posts() {
  const navigate = useNavigate();
  const { locale, t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState({ list: [], page: 1, page_size: 20, total: 0 });
  const [categories, setCategories] = useState([]);
  const [filters, setFilters] = useState({
    keyword: '',
    status: '',
    category_id: '',
    sort_by: 'updated_at',
    sort_order: 'desc',
  });

  async function loadCategories() {
    const next = await fetchCategories();
    setCategories(next?.list || []);
  }

  async function loadArticles(nextPage = 1, nextPageSize = result.page_size, nextFilters = filters) {
    setLoading(true);
    try {
      const payload = await fetchArticles({
        page: nextPage,
        page_size: nextPageSize,
        ...nextFilters,
      });
      setResult(payload || { list: [], page: 1, page_size: nextPageSize, total: 0 });
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadCategories().catch(() => setCategories([]));
    loadArticles().catch(() => setResult({ list: [], page: 1, page_size: 20, total: 0 }));
  }, []);

  const pagination = useMemo(() => {
    const page = result.page || 1;
    const totalPages = Math.max(1, Math.ceil((result.total || 0) / (result.page_size || 20)));
    return { page, totalPages };
  }, [result]);

  async function handleDelete(id) {
    await deleteArticle(id);
    await loadArticles(result.page || 1, result.page_size || 20, filters);
  }

  function statusLabel(status) {
    return status === 'published' ? t('common.statusPublished') : t('common.statusDraft');
  }

  const rows = result.list || [];
  const showingFrom = (pagination.page - 1) * (result.page_size || 20) + (rows.length ? 1 : 0);
  const showingTo = (pagination.page - 1) * (result.page_size || 20) + rows.length;

  return (
    <div className="admin-page">
      <section className="admin-page__header admin-page__header--split">
        <div>
          <h2>{t('posts.pageTitle')}</h2>
          <p>{t('posts.pageDesc')}</p>
        </div>
        <button type="button" className="admin-primary-button" onClick={() => navigate('/articles/new')}>
          <AdminIcon name="add" />
          <span>{t('common.newPost')}</span>
        </button>
      </section>

      <section className="admin-panel">
        <div className="admin-filter-bar">
          <div className="admin-search-field">
            <AdminIcon name="search" />
            <input
              value={filters.keyword}
              onChange={(event) => setFilters((prev) => ({ ...prev, keyword: event.target.value }))}
              placeholder={t('posts.searchPlaceholder')}
            />
          </div>

          <div className="admin-filter-bar__group">
            <select value={filters.status} onChange={(event) => setFilters((prev) => ({ ...prev, status: event.target.value }))}>
              <option value="">{t('posts.statusFilter')}</option>
              <option value="published">{t('common.statusPublished')}</option>
              <option value="draft">{t('common.statusDraft')}</option>
            </select>
            <select
              value={filters.category_id}
              onChange={(event) => setFilters((prev) => ({ ...prev, category_id: event.target.value }))}
            >
              <option value="">{t('posts.categoryFilter')}</option>
              {categories.map((item) => (
                <option key={item.id} value={item.id}>
                  {categoryDisplayName(t, item)}
                </option>
              ))}
            </select>
            <button type="button" className="admin-icon-button" onClick={() => loadArticles(1, result.page_size || 20, filters)} aria-label={t('common.search')}>
              <AdminIcon name="tune" />
            </button>
          </div>
        </div>

        <div className="admin-list-table admin-list-table--posts">
          <div className="admin-list-table__head admin-post-grid">
            <span>{t('posts.titleColumn')}</span>
            <span>{t('posts.statusColumn')}</span>
            <span>{t('posts.categoryColumn')}</span>
            <span>{t('posts.authorColumn')}</span>
            <span>{t('posts.dateColumn')}</span>
            <span className="align-right">{t('posts.actionsColumn')}</span>
          </div>

          {rows.map((item) => (
            <div key={item.id} className="admin-list-table__row admin-post-grid">
              <div className="admin-post-title">
                <div className="admin-post-title__thumb">
                  {item.cover_image ? <img src={item.cover_image} alt={item.title} /> : null}
                </div>
                <div>
                  <strong>{item.title}</strong>
                  <p>{item.slug}</p>
                </div>
              </div>
              <div>
                <span className={`admin-status-pill ${item.status === 'published' ? 'is-published' : 'is-draft'}`}>
                  {statusLabel(item.status)}
                </span>
              </div>
              <div>
                <span className="admin-category-pill">{categoryDisplayName(t, item.category) || t('common.uncategorized')}</span>
              </div>
              <span>{item.author?.username || t('common.admin')}</span>
              <span>{formatDateTime(item.published_at || item.updated_at, locale)}</span>
              <div className="admin-row-actions">
                <button type="button" className="admin-icon-button" onClick={() => navigate(`/articles/${item.id}`)} aria-label={t('common.edit')}>
                  <AdminIcon name="edit" />
                </button>
                <button type="button" className="admin-icon-button is-danger" onClick={() => handleDelete(item.id)} aria-label={t('common.delete')}>
                  <AdminIcon name="delete" />
                </button>
              </div>
            </div>
          ))}

          {loading ? <div className="admin-list-table__empty">{t('posts.loading')}</div> : null}
          {!loading && rows.length === 0 ? <div className="admin-list-table__empty">{t('posts.empty')}</div> : null}
        </div>

        <div className="admin-pagination">
          <p>{t('posts.showingRange', { from: showingFrom, to: showingTo, total: result.total || 0 })}</p>
          <div className="admin-pagination__controls">
            <button
              type="button"
              className="admin-icon-button"
              disabled={pagination.page <= 1}
              onClick={() => loadArticles(pagination.page - 1, result.page_size || 20, filters)}
            >
              <AdminIcon name="chevron_left" />
            </button>
            <span className="admin-pagination__current">{pagination.page}</span>
            <button
              type="button"
              className="admin-icon-button"
              disabled={pagination.page >= pagination.totalPages}
              onClick={() => loadArticles(pagination.page + 1, result.page_size || 20, filters)}
            >
              <AdminIcon name="chevron_right" />
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
