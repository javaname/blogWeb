import { useEffect, useMemo, useState } from 'react';
import AdminIcon from '../components/AdminIcon';
import { useI18n } from '../contexts/I18nContext';
import { deleteComment, fetchComments, updateCommentStatus } from '../utils/adminApi';
import { formatDateTime } from '../utils/format';

const initialFilters = {
  keyword: '',
  status: '',
};

export default function Comments() {
  const { locale, t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState({ list: [], page: 1, page_size: 20, total: 0 });
  const [filters, setFilters] = useState(initialFilters);
  const [message, setMessage] = useState('');

  async function loadComments(nextPage = 1, nextPageSize = result.page_size, nextFilters = filters) {
    setLoading(true);
    try {
      const payload = await fetchComments({
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
    loadComments().catch(() => setResult({ list: [], page: 1, page_size: 20, total: 0 }));
  }, []);

  const pagination = useMemo(() => {
    const page = result.page || 1;
    const totalPages = Math.max(1, Math.ceil((result.total || 0) / (result.page_size || 20)));
    return { page, totalPages };
  }, [result]);

  function statusLabel(status) {
    if (status === 'approved') {
      return t('comments.statusApproved');
    }
    if (status === 'rejected') {
      return t('comments.statusRejected');
    }
    return t('comments.statusPending');
  }

  async function handleStatus(id, status) {
    const rejectionReason =
      status === 'rejected'
        ? window.prompt(t('comments.rejectPrompt'), t('comments.defaultRejectReason')) || t('comments.defaultRejectReason')
        : '';
    await updateCommentStatus(id, { status, rejection_reason: rejectionReason });
    setMessage(status === 'approved' ? t('comments.approvedMessage') : t('comments.rejectedMessage'));
    await loadComments(result.page || 1, result.page_size || 20, filters);
  }

  async function handleDelete(id) {
    if (!window.confirm(t('comments.deleteConfirm'))) {
      return;
    }
    await deleteComment(id);
    setMessage(t('comments.deletedMessage'));
    await loadComments(result.page || 1, result.page_size || 20, filters);
  }

  const rows = result.list || [];
  const showingFrom = (pagination.page - 1) * (result.page_size || 20) + (rows.length ? 1 : 0);
  const showingTo = (pagination.page - 1) * (result.page_size || 20) + rows.length;

  return (
    <div className="admin-page">
      <section className="admin-page__header">
        <h2>{t('comments.pageTitle')}</h2>
        <p>{t('comments.pageDesc')}</p>
      </section>

      <section className="admin-panel admin-panel__head--stacked">
        <div className="admin-panel__head admin-panel__head--stacked">
          <div>
            <h3>{t('comments.policyTitle')}</h3>
            <p>{t('comments.policyDesc')}</p>
          </div>
        </div>
      </section>

      {message ? <div className="admin-inline-banner">{message}</div> : null}

      <section className="admin-panel">
        <div className="admin-panel__head">
          <div>
            <h3>{t('comments.queueTitle')}</h3>
            <p>{t('comments.queueDesc')}</p>
          </div>
        </div>

        <div className="admin-filter-bar">
          <div className="admin-search-field">
            <AdminIcon name="search" />
            <input
              value={filters.keyword}
              onChange={(event) => setFilters((prev) => ({ ...prev, keyword: event.target.value }))}
              placeholder={t('comments.searchPlaceholder')}
            />
          </div>

          <div className="admin-filter-bar__group">
            <select value={filters.status} onChange={(event) => setFilters((prev) => ({ ...prev, status: event.target.value }))}>
              <option value="">{t('comments.allStatuses')}</option>
              <option value="approved">{t('comments.statusApproved')}</option>
              <option value="pending">{t('comments.statusPending')}</option>
              <option value="rejected">{t('comments.statusRejected')}</option>
            </select>
            <button type="button" className="admin-icon-button" onClick={() => loadComments(1, result.page_size || 20, filters)} aria-label={t('common.search')}>
              <AdminIcon name="tune" />
            </button>
          </div>
        </div>

        <div className="admin-list-table admin-list-table--comments">
          <div className="admin-list-table__head admin-comment-grid">
            <span>{t('comments.authorColumn')}</span>
            <span>{t('comments.articleColumn')}</span>
            <span>{t('comments.contentColumn')}</span>
            <span>{t('comments.statusColumn')}</span>
            <span>{t('comments.dateColumn')}</span>
            <span className="align-right">{t('comments.actionsColumn')}</span>
          </div>

          {rows.map((item) => (
            <div key={item.id} className="admin-list-table__row admin-comment-grid">
              <strong>{item.author_name}</strong>
              <span>{item.article_title || t('comments.unknownArticle')}</span>
              <div>
                <p className="admin-comment-content">{item.content}</p>
                {item.rejection_reason ? <p className="admin-comment-reason">{item.rejection_reason}</p> : null}
              </div>
              <span className={`admin-status-pill is-${item.status}`}>{statusLabel(item.status)}</span>
              <span>{formatDateTime(item.created_at, locale)}</span>
              <div className="admin-row-actions">
                {item.status !== 'approved' ? (
                  <button type="button" className="admin-secondary-button admin-secondary-button--small" onClick={() => handleStatus(item.id, 'approved')}>
                    {t('comments.approve')}
                  </button>
                ) : null}
                {item.status !== 'rejected' ? (
                  <button type="button" className="admin-secondary-button admin-secondary-button--small" onClick={() => handleStatus(item.id, 'rejected')}>
                    {t('comments.reject')}
                  </button>
                ) : null}
                <button type="button" className="admin-icon-button is-danger" onClick={() => handleDelete(item.id)} aria-label={t('common.delete')}>
                  <AdminIcon name="delete" />
                </button>
              </div>
            </div>
          ))}

          {loading ? <div className="admin-list-table__empty">{t('comments.loading')}</div> : null}
          {!loading && rows.length === 0 ? <div className="admin-list-table__empty">{t('comments.empty')}</div> : null}
        </div>

        <div className="admin-pagination">
          <p>{t('comments.showingRange', { from: showingFrom, to: showingTo, total: result.total || 0 })}</p>
          <div className="admin-pagination__controls">
            <button
              type="button"
              className="admin-icon-button"
              disabled={pagination.page <= 1}
              onClick={() => loadComments(pagination.page - 1, result.page_size || 20, filters)}
            >
              <AdminIcon name="chevron_left" />
            </button>
            <span className="admin-pagination__current">{pagination.page}</span>
            <button
              type="button"
              className="admin-icon-button"
              disabled={pagination.page >= pagination.totalPages}
              onClick={() => loadComments(pagination.page + 1, result.page_size || 20, filters)}
            >
              <AdminIcon name="chevron_right" />
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
