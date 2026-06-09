use axum::{
    http::{
        header::{CONTENT_TYPE, COOKIE, SET_COOKIE},
        HeaderValue, Request,
    },
    middleware::{self, Next},
    response::{Redirect, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::admin_auth::{
    csrf_token, current_user, login, logout, register_with_email, request_registration_code,
};
use crate::admin_read::{
    dashboard, get_article as get_admin_article, list_articles as list_admin_articles,
    list_categories, list_comments, settings,
};
use crate::admin_users::{
    create_user, delete_user, get_user, list_users as list_admin_users, update_role_permissions,
    update_user, update_user_role,
};
use crate::admin_write::{
    create_article, create_category, delete_article, delete_category, delete_comment,
    sort_categories, update_article, update_category, update_comment_status, update_settings,
    upload,
};
use crate::config::Config;
use crate::db::DbPool;
use crate::http_interactions::{
    batch_likes, bookmark_article, create_comment, follow_author, like_article,
    subscribe_newsletter,
};
use crate::http_public::{
    about_page, archive_page, article_detail, article_page, author_page, categories_index_page,
    category_page, home_page, list_articles, not_found_page, search_page, serve_admin_app,
    serve_admin_asset, serve_asset, serve_upload, tag_page, PublicState,
};
use crate::session::RedisSessionStore;

static ANONYMOUS_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .layer(middleware::from_fn(apply_response_contract))
}

pub fn router_with_pool(pool: DbPool) -> Router {
    router_with_pool_and_config(
        pool,
        PathBuf::from("public/assets"),
        PathBuf::from("public/uploads"),
        Config::default(),
    )
}

pub fn router_with_pool_and_paths(
    pool: DbPool,
    assets_dir: impl Into<PathBuf>,
    upload_dir: impl Into<PathBuf>,
) -> Router {
    router_with_pool_and_config(pool, assets_dir, upload_dir, Config::default())
}

pub fn router_with_pool_and_config(
    pool: DbPool,
    assets_dir: impl Into<PathBuf>,
    upload_dir: impl Into<PathBuf>,
    config: Config,
) -> Router {
    let assets_dir = assets_dir.into();
    let admin_dir = assets_dir
        .parent()
        .map(|parent| parent.join("admin"))
        .unwrap_or_else(|| PathBuf::from("public/admin"));
    let state = PublicState {
        db: pool,
        assets_dir,
        admin_dir,
        upload_dir: upload_dir.into(),
        session_store: RedisSessionStore::new(&config),
        config,
    };
    Router::new()
        .route("/healthz", get(healthz))
        .route("/", get(home_page))
        .route("/search", get(search_page))
        .route("/about", get(about_page))
        .route("/authors/:id", get(author_page))
        .route("/archive", get(archive_page))
        .route("/tags/:slug", get(tag_page))
        .route("/categories", get(categories_index_page))
        .route("/articles/:slug", get(article_page))
        .route("/categories/:slug", get(category_page))
        .route("/assets/*path", get(serve_asset))
        .route("/admin", get(admin_redirect))
        .route("/admin/", get(serve_admin_app))
        .route("/admin/assets/*path", get(serve_admin_asset))
        .route("/admin/*path", get(serve_admin_app))
        .route("/uploads/*path", get(serve_upload))
        .route("/api/articles", get(list_articles))
        .route("/api/articles/:slug", get(article_detail))
        .route("/api/articles/:slug/like", post(like_article))
        .route("/api/articles/:slug/bookmark", post(bookmark_article))
        .route("/api/articles/:slug/comments", post(create_comment))
        .route("/api/authors/:id/follow", post(follow_author))
        .route("/api/newsletter/subscribe", post(subscribe_newsletter))
        .route("/api/likes/batch", post(batch_likes))
        .route("/api/auth/register/code", post(request_registration_code))
        .route("/api/auth/register", post(register_with_email))
        .route("/api/admin/login", post(login))
        .route("/api/admin/logout", post(logout))
        .route("/api/admin/csrf-token", get(csrf_token))
        .route("/api/admin/me", get(current_user))
        .route("/api/admin/dashboard", get(dashboard))
        .route("/api/admin/settings", get(settings).put(update_settings))
        .route("/api/admin/role-permissions", put(update_role_permissions))
        .route("/api/admin/users", get(list_admin_users).post(create_user))
        .route("/api/admin/users/:id/role", put(update_user_role))
        .route(
            "/api/admin/users/:id",
            get(get_user).put(update_user).delete(delete_user),
        )
        .route(
            "/api/admin/articles",
            get(list_admin_articles).post(create_article),
        )
        .route(
            "/api/admin/articles/:id",
            get(get_admin_article)
                .put(update_article)
                .delete(delete_article),
        )
        .route(
            "/api/admin/categories",
            get(list_categories).post(create_category),
        )
        .route("/api/admin/categories/sort", put(sort_categories))
        .route(
            "/api/admin/categories/:id",
            put(update_category).delete(delete_category),
        )
        .route("/api/admin/comments", get(list_comments))
        .route("/api/admin/comments/:id/status", put(update_comment_status))
        .route("/api/admin/comments/:id", delete(delete_comment))
        .route("/api/admin/upload", post(upload))
        .fallback(get(not_found_page))
        .with_state(state)
        .layer(middleware::from_fn(apply_response_contract))
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn admin_redirect() -> Redirect {
    Redirect::temporary("/admin/")
}

async fn apply_response_contract(request: Request<axum::body::Body>, next: Next) -> Response {
    let has_cookie = request
        .headers()
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|cookies| {
            cookies
                .split(';')
                .any(|part| part.trim_start().starts_with("anonymous_id="))
        });
    let mut response = next.run(request).await;
    match response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        Some(value) if value.starts_with("application/json") => {
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            );
        }
        Some(_) => {}
        None => {
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            );
        }
    }
    response.headers_mut().insert(
        "Content-Security-Policy",
        HeaderValue::from_static(
            "default-src 'self'; base-uri 'self'; connect-src 'self'; img-src 'self' data: https:; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' data: https://fonts.gstatic.com; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com; object-src 'none'; frame-ancestors 'none'",
        ),
    );
    response.headers_mut().insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
        .headers_mut()
        .insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    if !has_cookie {
        let cookie = format!(
            "anonymous_id={}; Path=/; Max-Age=31536000; HttpOnly",
            anonymous_id()
        );
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            response.headers_mut().append(SET_COOKIE, value);
        }
    }
    response
}

fn anonymous_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = ANONYMOUS_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:032x}{counter:016x}")
}
