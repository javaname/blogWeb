import { DEFAULT_LOCALE } from '../i18n/messages';
import { translateMessage } from '../i18n/translate';

const API_BASE = '/api';
let csrfToken = '';
let unauthorizedHandler = null;
let forbiddenHandler = null;
let errorMessageResolver = null;

export class ApiError extends Error {
  constructor(message, status, payload) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.payload = payload;
  }
}

export function setCsrfToken(token) {
  csrfToken = token || '';
}

export function getCsrfToken() {
  return csrfToken;
}

export function clearCsrfToken() {
  csrfToken = '';
}

export function configureApiHandlers({ onUnauthorized, onForbidden } = {}) {
  unauthorizedHandler = onUnauthorized || null;
  forbiddenHandler = onForbidden || null;
}

export function configureApiMessages({ resolveErrorMessage } = {}) {
  errorMessageResolver = resolveErrorMessage || null;
}

function showToast(type, content) {
  const toast = window.__BLOG_ADMIN_TOAST__;
  if (toast?.[type]) {
    toast[type](content);
  }
}

function buildHeaders(extraHeaders, body) {
  const headers = new Headers(extraHeaders || {});
  if (!(body instanceof FormData) && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }
  if (csrfToken && !headers.has('X-CSRF-Token')) {
    headers.set('X-CSRF-Token', csrfToken);
  }
  return headers;
}

async function parseBody(response) {
  const contentType = response.headers.get('content-type') || '';
  if (contentType.includes('application/json')) {
    return response.json();
  }
  const text = await response.text();
  return text ? { message: text } : null;
}

function resolveErrorMessage(payload, status) {
  if (errorMessageResolver) {
    const resolved = errorMessageResolver(payload, status);
    if (resolved) {
      return resolved;
    }
  }
  return payload?.message || translateMessage(DEFAULT_LOCALE, 'errors.requestFailed', { status });
}

export async function apiRequest(path, options = {}) {
  const { body, headers, skipErrorToast = false, ...rest } = options;
  const response = await fetch(`${API_BASE}${path}`, {
    credentials: 'include',
    ...rest,
    headers: buildHeaders(headers, body),
    body:
      body == null
        ? undefined
        : body instanceof FormData
          ? body
          : JSON.stringify(body),
  });

  const payload = await parseBody(response);

  if (response.ok) {
    return payload;
  }

  if (response.status === 401) {
    clearCsrfToken();
    unauthorizedHandler?.(payload);
  } else if (response.status === 403) {
    forbiddenHandler?.(payload);
  }

  const message = resolveErrorMessage(payload, response.status);
  if (!skipErrorToast && response.status !== 401) {
    showToast('error', message);
  }

  throw new ApiError(message, response.status, payload);
}
