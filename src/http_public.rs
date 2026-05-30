use axum::{
    extract::{Path, Query, State},
    http::{
        header::{CONTENT_TYPE, LOCATION},
        HeaderValue, StatusCode,
    },
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, QueryBuilder, Row, Sqlite};
use std::collections::HashMap;
use std::path::{Component, Path as FsPath, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::fs;

use crate::{
    config::Config,
    error::{AppError, Result},
    renderer,
};

#[derive(Clone)]
pub struct PublicState {
    pub db: Pool<Sqlite>,
    pub assets_dir: PathBuf,
    pub upload_dir: PathBuf,
    pub sessions: Arc<RwLock<HashMap<String, crate::admin_auth::SessionUser>>>,
    pub config: Config,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    limit: Option<i64>,
    category: Option<String>,
    keyword: Option<String>,
    cursor: Option<String>,
}

impl Default for ListQuery {
    fn default() -> Self {
        Self {
            limit: Some(12),
            category: None,
            keyword: None,
            cursor: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Cursor {
    is_pinned: i64,
    published_at: String,
    id: i64,
}

#[derive(Debug, Serialize)]
struct ListPublishedResult {
    list: Vec<PublicArticleSummary>,
    next_cursor: String,
    has_more: bool,
}

#[derive(Debug, Serialize)]
struct PublicArticleSummary {
    id: i64,
    title: String,
    slug: String,
    cover_image: String,
    excerpt: String,
    category: Option<PublicCategory>,
    author: PublicAuthor,
    is_pinned: bool,
    like_count: i64,
    read_time_min: i64,
    published_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct PublicArticleDetail {
    #[serde(flatten)]
    summary: PublicArticleSummary,
    content_html: String,
    user_liked: bool,
    user_bookmarked: bool,
    author_followed: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct PublicCategory {
    id: i64,
    name: String,
    slug: String,
}

#[derive(Debug, Serialize)]
struct PublicAuthor {
    id: i64,
    username: String,
}

pub async fn home_page(State(state): State<PublicState>) -> Result<axum::response::Response> {
    let articles = published_summaries(&state, ListQuery::default()).await?;
    let mut html = String::from("<!doctype html><html lang=\"zh-CN\"><body><main>");
    for article in articles.list {
        html.push_str("<article><a href=\"/articles/");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\">");
        html.push_str(&escape_html(&article.title));
        html.push_str("</a></article>");
    }
    html.push_str("</main></body></html>");
    Ok(html_response(html))
}

pub async fn article_page(
    State(state): State<PublicState>,
    Path(slug): Path<String>,
) -> Result<axum::response::Response> {
    let detail = published_detail(&state.db, &slug).await?;
    let Some(detail) = detail else {
        return Err(AppError::HttpStatus(404, "not_found".into()));
    };
    let mut html = String::from("<!doctype html><html lang=\"zh-CN\"><body><article>");
    html.push_str("<h1>");
    html.push_str(&escape_html(&detail.summary.title));
    html.push_str("</h1><div class=\"article-html\">");
    html.push_str(&detail.content_html);
    html.push_str("</div></article></body></html>");
    Ok(html_response(html))
}

pub async fn category_page(
    State(state): State<PublicState>,
    Path(slug): Path<String>,
) -> Result<axum::response::Response> {
    let category = sqlx::query("SELECT id, name, slug FROM categories WHERE slug = ?")
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::HttpStatus(404, "not_found".into()))?;
    let category_name: String = category.try_get("name")?;
    let articles = published_summaries(
        &state,
        ListQuery {
            category: Some(slug),
            ..ListQuery::default()
        },
    )
    .await?;
    let mut html = String::from("<!doctype html><html lang=\"zh-CN\"><body><main><h1>");
    html.push_str(&escape_html(&category_name));
    html.push_str("</h1>");
    for article in articles.list {
        html.push_str("<article>");
        html.push_str(&escape_html(&article.title));
        html.push_str("</article>");
    }
    html.push_str("</main></body></html>");
    Ok(html_response(html))
}

pub async fn serve_asset(
    State(state): State<PublicState>,
    Path(path): Path<String>,
) -> Result<axum::response::Response> {
    serve_file(state.assets_dir, path).await
}

pub async fn serve_upload(
    State(state): State<PublicState>,
    Path(path): Path<String>,
) -> Result<axum::response::Response> {
    serve_file(state.upload_dir, path).await
}

pub async fn list_articles(
    State(state): State<PublicState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::to_value(
        published_summaries(&state, query).await?,
    )?))
}

async fn published_summaries(state: &PublicState, query: ListQuery) -> Result<ListPublishedResult> {
    let limit = query.limit.unwrap_or(12).clamp(1, 50);
    let cursor = match query
        .cursor
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(value) => Some(serde_json::from_str::<Cursor>(value)?),
        None => None,
    };
    let mut builder = QueryBuilder::new(
        "SELECT
            articles.id,
            articles.title,
            articles.slug,
            articles.cover_image,
            articles.excerpt,
            articles.is_pinned,
            articles.published_at,
            categories.id AS category_id,
            categories.name AS category_name,
            categories.slug AS category_slug,
            users.id AS author_id,
            users.username AS author_username,
            COUNT(likes.id) AS like_count
         FROM articles
         LEFT JOIN categories ON categories.id = articles.category_id
         INNER JOIN users ON users.id = articles.author_id
         LEFT JOIN likes ON likes.article_id = articles.id
         WHERE articles.status = 'published'
           AND (articles.published_at IS NULL OR articles.published_at <= datetime('now'))",
    );
    if let Some(category) = query
        .category
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        builder.push(" AND categories.slug = ");
        builder.push_bind(category);
    }
    if let Some(keyword) = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        let pattern = format!("%{keyword}%");
        builder.push(" AND (articles.title LIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR articles.excerpt LIKE ");
        builder.push_bind(pattern);
        builder.push(")");
    }
    if let Some(cursor) = &cursor {
        builder.push(" AND (articles.is_pinned < ");
        builder.push_bind(cursor.is_pinned);
        builder.push(" OR (articles.is_pinned = ");
        builder.push_bind(cursor.is_pinned);
        builder.push(" AND articles.published_at < ");
        builder.push_bind(cursor.published_at.clone());
        builder.push(") OR (articles.is_pinned = ");
        builder.push_bind(cursor.is_pinned);
        builder.push(" AND articles.published_at = ");
        builder.push_bind(cursor.published_at.clone());
        builder.push(" AND articles.id < ");
        builder.push_bind(cursor.id);
        builder.push("))");
    }
    builder.push(
        " GROUP BY articles.id
          ORDER BY articles.is_pinned DESC, articles.published_at DESC, articles.id DESC
          LIMIT ",
    );
    builder.push_bind(limit + 1);
    let mut rows = builder.build().fetch_all(&state.db).await?;
    let has_more = rows.len() as i64 > limit;
    if has_more {
        rows.truncate(limit as usize);
    }

    let list = rows
        .into_iter()
        .map(|row| summary_from_row(&row))
        .collect::<Result<Vec<_>>>()?;
    let next_cursor = if has_more {
        list.last()
            .and_then(|item| {
                serde_json::to_string(&Cursor {
                    is_pinned: i64::from(item.is_pinned),
                    published_at: item.published_at.clone().unwrap_or_default(),
                    id: item.id,
                })
                .ok()
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    Ok(ListPublishedResult {
        list,
        next_cursor,
        has_more,
    })
}

pub async fn article_detail(
    State(state): State<PublicState>,
    Path(slug): Path<String>,
) -> Result<axum::response::Response> {
    let Some(detail) = published_detail(&state.db, &slug).await? else {
        if let Some(current_slug) = lookup_current_slug(&state.db, &slug).await? {
            let location = format!("/api/articles/{current_slug}");
            let mut response = StatusCode::MOVED_PERMANENTLY.into_response();
            response.headers_mut().insert(
                LOCATION,
                HeaderValue::from_str(&location)
                    .map_err(|err| AppError::Config(err.to_string()))?,
            );
            return Ok(response);
        }
        return Err(AppError::HttpStatus(404, "not_found".into()));
    };
    Ok(Json(serde_json::to_value(detail)?).into_response())
}

async fn published_detail(pool: &Pool<Sqlite>, slug: &str) -> Result<Option<PublicArticleDetail>> {
    let row = sqlx::query(
        "SELECT
            articles.id,
            articles.title,
            articles.slug,
            articles.content,
            articles.cover_image,
            articles.excerpt,
            articles.is_pinned,
            articles.published_at,
            articles.created_at,
            articles.updated_at,
            categories.id AS category_id,
            categories.name AS category_name,
            categories.slug AS category_slug,
            users.id AS author_id,
            users.username AS author_username,
            COUNT(likes.id) AS like_count
         FROM articles
         LEFT JOIN categories ON categories.id = articles.category_id
         INNER JOIN users ON users.id = articles.author_id
         LEFT JOIN likes ON likes.article_id = articles.id
         WHERE articles.slug = ?
           AND articles.status = 'published'
           AND (articles.published_at IS NULL OR articles.published_at <= datetime('now'))
         GROUP BY articles.id",
    )
    .bind(&slug)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    let summary = summary_from_row(&row)?;
    Ok(Some(PublicArticleDetail {
        summary,
        content_html: renderer::render_safe_html(&row.try_get::<String, _>("content")?)?.0,
        user_liked: false,
        user_bookmarked: false,
        author_followed: false,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    }))
}

async fn lookup_current_slug(pool: &Pool<Sqlite>, old_slug: &str) -> Result<Option<String>> {
    let slug = sqlx::query_scalar(
        "SELECT articles.slug
         FROM slug_history
         INNER JOIN articles ON articles.id = slug_history.article_id
         WHERE slug_history.old_slug = ?",
    )
    .bind(old_slug)
    .fetch_optional(pool)
    .await?;
    Ok(slug)
}

fn summary_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<PublicArticleSummary> {
    let category_id: Option<i64> = row.try_get("category_id")?;
    let category = match category_id {
        Some(id) => Some(PublicCategory {
            id,
            name: row.try_get("category_name")?,
            slug: row.try_get("category_slug")?,
        }),
        None => None,
    };
    let content: Option<String> = row.try_get("content").ok();
    let excerpt: String = row.try_get("excerpt")?;
    Ok(PublicArticleSummary {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        slug: row.try_get("slug")?,
        cover_image: row.try_get("cover_image")?,
        excerpt,
        category,
        author: PublicAuthor {
            id: row.try_get("author_id")?,
            username: row.try_get("author_username")?,
        },
        is_pinned: row.try_get::<i64, _>("is_pinned")? != 0,
        like_count: row.try_get("like_count")?,
        read_time_min: content.as_deref().map(read_time_min).unwrap_or(1),
        published_at: row.try_get("published_at")?,
    })
}

fn read_time_min(content: &str) -> i64 {
    let words = content.split_whitespace().count() as i64;
    (words / 300).max(1)
}

fn html_response(html: String) -> axum::response::Response {
    let mut response = html.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
}

async fn serve_file(root: PathBuf, path: String) -> Result<axum::response::Response> {
    let Some(path) = clean_relative_path(&path) else {
        return Err(AppError::HttpStatus(404, "not_found".into()));
    };
    let full_path = root.join(path);
    let data = fs::read(&full_path)
        .await
        .map_err(|_| AppError::HttpStatus(404, "not_found".into()))?;
    let mut response = data.into_response();
    if let Some(content_type) = content_type_for(&full_path) {
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    }
    Ok(response)
}

fn clean_relative_path(path: &str) -> Option<PathBuf> {
    let path = FsPath::new(path);
    if path.is_absolute() {
        return None;
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => result.push(value),
            _ => return None,
        }
    }
    if result.as_os_str().is_empty() {
        None
    } else {
        Some(result)
    }
}

fn content_type_for(path: &FsPath) -> Option<&'static str> {
    match path.extension().and_then(|value| value.to_str()) {
        Some("css") => Some("text/css; charset=utf-8"),
        Some("js") => Some("application/javascript; charset=utf-8"),
        Some("html") => Some("text/html; charset=utf-8"),
        Some("json") => Some("application/json; charset=utf-8"),
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        Some("txt") => Some("text/plain; charset=utf-8"),
        _ => None,
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
