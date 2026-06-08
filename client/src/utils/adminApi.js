import { apiRequest } from './api';

export function login(payload) {
  return apiRequest('/admin/login', {
    method: 'POST',
    body: payload,
    skipErrorToast: true,
  });
}

export function requestRegistrationCode(payload) {
  return apiRequest('/auth/register/code', {
    method: 'POST',
    body: payload,
    skipErrorToast: true,
  });
}

export function registerWithEmail(payload) {
  return apiRequest('/auth/register', {
    method: 'POST',
    body: payload,
    skipErrorToast: true,
  });
}

export function logout() {
  return apiRequest('/admin/logout', {
    method: 'POST',
    body: {},
  });
}

export function fetchCsrfToken() {
  return apiRequest('/admin/csrf-token');
}

export function fetchDashboard() {
  return apiRequest('/admin/dashboard');
}

export function fetchSettings() {
  return apiRequest('/admin/settings');
}

export function updateSettings(payload) {
  return apiRequest('/admin/settings', {
    method: 'PUT',
    body: payload,
  });
}

export function fetchUsers() {
  return apiRequest('/admin/users');
}

export function fetchUser(id) {
  return apiRequest(`/admin/users/${id}`);
}

export function createUser(payload) {
  return apiRequest('/admin/users', {
    method: 'POST',
    body: payload,
  });
}

export function updateUser(id, payload) {
  return apiRequest(`/admin/users/${id}`, {
    method: 'PUT',
    body: payload,
  });
}

export function updateUserRole(id, payload) {
  return apiRequest(`/admin/users/${id}/role`, {
    method: 'PUT',
    body: payload,
  });
}

export function deleteUser(id) {
  return apiRequest(`/admin/users/${id}`, {
    method: 'DELETE',
  });
}

export function fetchArticles(params) {
  const search = new URLSearchParams();
  Object.entries(params || {}).forEach(([key, value]) => {
    if (value !== '' && value !== null && value !== undefined) {
      search.set(key, String(value));
    }
  });
  return apiRequest(`/admin/articles${search.toString() ? `?${search}` : ''}`);
}

export function fetchArticle(id) {
  return apiRequest(`/admin/articles/${id}`);
}

export function createArticle(payload) {
  return apiRequest('/admin/articles', {
    method: 'POST',
    body: payload,
  });
}

export function updateArticle(id, payload) {
  return apiRequest(`/admin/articles/${id}`, {
    method: 'PUT',
    body: payload,
  });
}

export function deleteArticle(id) {
  return apiRequest(`/admin/articles/${id}`, {
    method: 'DELETE',
  });
}

export function fetchCategories() {
  return apiRequest('/admin/categories');
}

export function fetchComments(params) {
  const search = new URLSearchParams();
  Object.entries(params || {}).forEach(([key, value]) => {
    if (value !== '' && value !== null && value !== undefined) {
      search.set(key, String(value));
    }
  });
  return apiRequest(`/admin/comments${search.toString() ? `?${search}` : ''}`);
}

export function updateCommentStatus(id, payload) {
  return apiRequest(`/admin/comments/${id}/status`, {
    method: 'PUT',
    body: payload,
  });
}

export function deleteComment(id) {
  return apiRequest(`/admin/comments/${id}`, {
    method: 'DELETE',
  });
}

export function createCategory(payload) {
  return apiRequest('/admin/categories', {
    method: 'POST',
    body: payload,
  });
}

export function updateCategory(id, payload) {
  return apiRequest(`/admin/categories/${id}`, {
    method: 'PUT',
    body: payload,
  });
}

export function deleteCategory(id) {
  return apiRequest(`/admin/categories/${id}`, {
    method: 'DELETE',
  });
}

export function sortCategories(ids) {
  return apiRequest('/admin/categories/sort', {
    method: 'PUT',
    body: { ids },
  });
}

export function uploadImage(file) {
  const formData = new FormData();
  formData.append('file', file);
  return apiRequest('/admin/upload', {
    method: 'POST',
    body: formData,
    headers: {},
  });
}
