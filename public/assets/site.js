(function () {
  function applyLikedState(button, liked) {
    const icon = button.querySelector('.like-icon');
    if (icon) {
      icon.textContent = liked ? 'favorite' : 'favorite_border';
    }
    if (liked) {
      button.classList.add('text-error', 'liked');
      button.classList.remove('text-on-surface-variant');
    } else {
      button.classList.remove('text-error', 'liked');
      button.classList.add('text-on-surface-variant');
    }
  }

  function updateLikeButton(button, liked, count) {
    applyLikedState(button, Boolean(liked));
    const countEl = button.querySelector('.like-count');
    if (countEl && typeof count === 'number') {
      countEl.textContent = String(count);
    }
  }

  async function postJSON(url, body) {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body || {}),
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok) {
      throw new Error(payload?.message || '操作失败，请稍后再试');
    }
    return payload;
  }

  async function bootstrapLikes() {
    const buttons = Array.from(document.querySelectorAll('[data-like-button]'));
    if (!buttons.length) {
      return;
    }

    const articleSlugs = buttons.map((button) => button.dataset.slug).filter(Boolean);
    if (articleSlugs.length) {
      try {
        const response = await fetch('/api/likes/batch', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ article_slugs: articleSlugs }),
        });
        if (response.ok) {
          const payload = await response.json();
          const likedMap = payload?.liked_map || {};
          buttons.forEach((button) => {
            const slug = button.dataset.slug;
            const currentCount = Number(button.querySelector('.like-count')?.textContent || 0);
            updateLikeButton(button, Boolean(likedMap[slug]), currentCount);
          });
        }
      } catch (_) {
        /* ignore network errors during bootstrap */
      }
    }

    buttons.forEach((button) => {
      button.addEventListener('click', async (event) => {
        event.preventDefault();
        event.stopPropagation();
        const currentlyLiked = button.classList.contains('liked');
        const action = currentlyLiked ? 'unlike' : 'like';
        const slug = button.dataset.slug;
        try {
          const response = await fetch(`/api/articles/${slug}/like`, {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({ action }),
          });
          const payload = await response.json();
          if (!response.ok) {
            throw new Error(payload?.message || 'Operation failed');
          }
          updateLikeButton(button, payload.liked, payload.like_count);
        } catch (error) {
          window.alert(error.message || 'Operation failed');
        }
      });
    });
  }

  function setCommentMessage(form, message, isError) {
    const messageEl = form.querySelector('[data-comment-message]');
    if (!messageEl) {
      return;
    }
    messageEl.textContent = message || '';
    messageEl.classList.toggle('text-error', Boolean(isError));
    messageEl.classList.toggle('text-primary', Boolean(message && !isError));
  }

  function initComments() {
    const forms = Array.from(document.querySelectorAll('[data-comment-form]'));
    if (!forms.length) {
      return;
    }
    forms.forEach((form) => {
      const parentInput = form.querySelector('[data-comment-parent-id]');
      const replyTarget = form.querySelector('[data-reply-target]');
      const replyAuthor = form.querySelector('[data-reply-author]');
      const cancelReply = form.querySelector('[data-reply-cancel]');

      function clearReplyTarget() {
        if (parentInput) {
          parentInput.value = '';
        }
        if (replyTarget) {
          replyTarget.classList.add('hidden');
        }
        if (replyAuthor) {
          replyAuthor.textContent = '';
        }
      }

      cancelReply?.addEventListener('click', clearReplyTarget);

      form.addEventListener('submit', async (event) => {
        event.preventDefault();
        const slug = form.dataset.slug;
        const submitButton = form.querySelector('button[type="submit"]');
        const authorName = form.elements.author_name?.value || '';
        const content = form.elements.content?.value || '';
        const parentId = parentInput?.value ? Number(parentInput.value) : undefined;
        const requestBody = { author_name: authorName, content };
        if (parentId) {
          requestBody.parent_id = parentId;
        }
        setCommentMessage(form, '', false);
        if (submitButton) {
          submitButton.disabled = true;
          submitButton.classList.add('opacity-70', 'cursor-wait');
        }
        try {
          const response = await fetch(`/api/articles/${slug}/comments`, {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify(requestBody),
          });
          const payload = await response.json();
          if (!response.ok) {
            throw new Error(payload?.message || '评论提交失败，请稍后再试');
          }
          form.reset();
          clearReplyTarget();
          setCommentMessage(form, payload?.message || '评论已发布', false);
          window.setTimeout(() => window.location.reload(), 500);
        } catch (error) {
          setCommentMessage(form, error.message || '评论提交失败，请稍后再试', true);
        } finally {
          if (submitButton) {
            submitButton.disabled = false;
            submitButton.classList.remove('opacity-70', 'cursor-wait');
          }
        }
      });
    });

    document.querySelectorAll('[data-comment-reply]').forEach((button) => {
      button.addEventListener('click', () => {
        const form = document.querySelector('[data-comment-form]');
        if (!form) {
          return;
        }
        const parentInput = form.querySelector('[data-comment-parent-id]');
        const replyTarget = form.querySelector('[data-reply-target]');
        const replyAuthor = form.querySelector('[data-reply-author]');
        if (parentInput) {
          parentInput.value = button.dataset.commentId || '';
        }
        if (replyAuthor) {
          replyAuthor.textContent = button.dataset.commentAuthor || '这条评论';
        }
        if (replyTarget) {
          replyTarget.classList.remove('hidden');
        }
        form.scrollIntoView({ behavior: 'smooth', block: 'center' });
        window.setTimeout(() => form.elements.content?.focus(), 300);
      });
    });
  }

  function initReadingProgress() {
    if (document.body.dataset.page !== 'article') {
      return;
    }
    const bar = document.getElementById('reading-progress');
    if (!bar) {
      return;
    }
    function update() {
      const scrollTop = window.scrollY || window.pageYOffset || 0;
      const docHeight = document.documentElement.scrollHeight - window.innerHeight;
      const ratio = docHeight > 0 ? Math.min(1, Math.max(0, scrollTop / docHeight)) : 0;
      bar.style.width = (ratio * 100).toFixed(2) + '%';
    }
    let ticking = false;
    window.addEventListener('scroll', () => {
      if (!ticking) {
        window.requestAnimationFrame(() => {
          update();
          ticking = false;
        });
        ticking = true;
      }
    }, { passive: true });
    window.addEventListener('resize', update);
    update();
  }

  function showPageMessage(message, isError) {
    let messageEl = document.querySelector('[data-page-message]');
    if (!messageEl) {
      messageEl = document.createElement('div');
      messageEl.setAttribute('data-page-message', '');
      messageEl.setAttribute('role', 'status');
      messageEl.className = 'fixed bottom-6 left-1/2 z-[100] max-w-[calc(100vw-32px)] -translate-x-1/2 rounded-lg bg-inverse-surface px-4 py-3 text-sm text-inverse-on-surface shadow-lg';
      document.body.appendChild(messageEl);
    }
    messageEl.textContent = message;
    messageEl.classList.toggle('bg-error', Boolean(isError));
    window.clearTimeout(showPageMessage.timer);
    showPageMessage.timer = window.setTimeout(() => {
      messageEl.remove();
    }, 2600);
  }

  function initSearch() {
    const toggle = document.querySelector('[data-search-toggle]');
    const panel = document.querySelector('[data-search-panel]');
    const form = document.querySelector('[data-search-form]');
    const input = form?.elements.keyword;

    if (toggle && panel) {
      toggle.addEventListener('click', () => {
        panel.classList.toggle('hidden');
        if (!panel.classList.contains('hidden')) {
          window.setTimeout(() => input?.focus(), 0);
        }
      });
    }

    if (!form) {
      return;
    }
    form.addEventListener('submit', (event) => {
      event.preventDefault();
      const keyword = String(input?.value || '').trim();
      if (!keyword) {
        showPageMessage('请输入关键词');
        input?.focus();
        return;
      }
      const target = new URL('/', window.location.origin);
      target.searchParams.set('keyword', keyword);
      window.location.href = target.pathname + target.search;
    });
  }

  function initFeedbackButtons() {
    document.querySelectorAll('[data-feedback-message]').forEach((button) => {
      button.addEventListener('click', () => {
        showPageMessage(button.dataset.feedbackMessage || '操作已记录');
      });
    });

    document.querySelectorAll('[data-share-page]').forEach((button) => {
      button.addEventListener('click', async () => {
        const url = window.location.href;
        const title = document.title;
        try {
          if (navigator.share) {
            await navigator.share({ title, url });
            return;
          }
          await navigator.clipboard.writeText(url);
          showPageMessage('链接已复制');
        } catch (_) {
          showPageMessage('分享暂时不可用，请手动复制地址栏链接', true);
        }
      });
    });
  }

  function initNewsletter() {
    document.querySelectorAll('[data-newsletter-form]').forEach((form) => {
      form.addEventListener('submit', async (event) => {
        event.preventDefault();
        const input = form.elements.email;
        const message = form.querySelector('[data-newsletter-message]');
        const submitButton = form.querySelector('button[type="submit"]');
        const email = String(input?.value || '').trim();
        if (!email) {
          if (message) {
            message.textContent = '请填写邮箱地址';
          }
          input?.focus();
          return;
        }
        if (submitButton) {
          submitButton.disabled = true;
          submitButton.classList.add('opacity-70', 'cursor-wait');
        }
        try {
          const payload = await postJSON('/api/newsletter/subscribe', { email });
          if (message) {
            message.textContent = `${payload.email || email} 已订阅。`;
            message.classList.remove('text-error');
          }
          form.reset();
        } catch (error) {
          if (message) {
            message.textContent = error.message || '订阅失败，请稍后再试';
            message.classList.add('text-error');
          }
        } finally {
          if (submitButton) {
            submitButton.disabled = false;
            submitButton.classList.remove('opacity-70', 'cursor-wait');
          }
        }
      });
    });

    document.querySelectorAll('[data-newsletter-focus]').forEach((button) => {
      button.addEventListener('click', () => {
        const form = document.querySelector('[data-newsletter-form]');
        if (!form) {
          window.location.href = '/#newsletter';
          return;
        }
        form.scrollIntoView({ behavior: 'smooth', block: 'center' });
        window.setTimeout(() => form.elements.email?.focus(), 300);
      });
    });
  }

  function applyBookmarkState(button, bookmarked) {
    const icon = button.querySelector('.bookmark-icon');
    if (icon) {
      icon.textContent = bookmarked ? 'bookmark' : 'bookmark_border';
    }
    button.classList.toggle('bookmarked', Boolean(bookmarked));
    button.classList.toggle('text-primary', Boolean(bookmarked));
    button.classList.toggle('text-on-surface-variant', !bookmarked);
  }

  function initBookmarks() {
    document.querySelectorAll('[data-bookmark-button]').forEach((button) => {
      button.addEventListener('click', async () => {
        const slug = button.dataset.slug;
        const action = button.classList.contains('bookmarked') ? 'unbookmark' : 'bookmark';
        button.disabled = true;
        try {
          const payload = await postJSON(`/api/articles/${slug}/bookmark`, { action });
          applyBookmarkState(button, payload.bookmarked);
          showPageMessage(payload.bookmarked ? '已收藏文章' : '已取消收藏');
        } catch (error) {
          showPageMessage(error.message || '收藏失败，请稍后再试', true);
        } finally {
          button.disabled = false;
        }
      });
    });
  }

  function applyFollowState(button, following) {
    button.dataset.following = following ? 'true' : 'false';
    const authorName = button.dataset.authorName || '';
    button.textContent = `${following ? '已关注' : '关注'} ${authorName}`.trim();
  }

  function initAuthorFollows() {
    document.querySelectorAll('[data-follow-author]').forEach((button) => {
      button.addEventListener('click', async () => {
        const authorId = button.dataset.authorId;
        const following = button.dataset.following === 'true';
        const action = following ? 'unfollow' : 'follow';
        button.disabled = true;
        try {
          const payload = await postJSON(`/api/authors/${authorId}/follow`, { action });
          applyFollowState(button, payload.following);
          showPageMessage(payload.following ? '已关注作者' : '已取消关注');
        } catch (error) {
          showPageMessage(error.message || '关注失败，请稍后再试', true);
        } finally {
          button.disabled = false;
        }
      });
    });
  }

  function init() {
    initSearch();
    initFeedbackButtons();
    initNewsletter();
    initBookmarks();
    initAuthorFollows();
    bootstrapLikes();
    initComments();
    initReadingProgress();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
