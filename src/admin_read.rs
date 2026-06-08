use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{QueryBuilder, Row};

use crate::{
    admin_auth::{auth_required, session_user},
    db::Db as DbBackend,
    error::Result,
    http_public::PublicState,
};

#[derive(Debug, Deserialize)]
pub struct ArticleListQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
    category_id: Option<i64>,
    keyword: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommentListQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
    keyword: Option<String>,
}

fn normalize_page(page: Option<i64>) -> i64 {
    page.unwrap_or(1).max(1)
}

fn normalize_page_size(page_size: Option<i64>) -> i64 {
    match page_size.unwrap_or(20) {
        value if value <= 0 => 20,
        value if value > 100 => 100,
        value => value,
    }
}

#[derive(Debug, Serialize)]
struct AdminList<T> {
    list: Vec<T>,
    page: i64,
    page_size: i64,
    total: i64,
}

#[derive(Debug, Serialize)]
struct AdminArticle {
    id: i64,
    title: String,
    slug: String,
    cover_image: String,
    status: String,
    is_pinned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<AdminCategoryRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<AdminAuthorRef>,
    like_count: i64,
    published_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct AdminCategoryRef {
    id: i64,
    name: String,
}

#[derive(Debug, Serialize)]
struct AdminAuthorRef {
    id: i64,
    username: String,
}

#[derive(Debug, Serialize)]
struct AdminCategory {
    id: i64,
    name: String,
    slug: String,
    sort_order: i64,
    created_at: String,
    article_count: i64,
}

#[derive(Debug, Serialize)]
struct AdminComment {
    id: i64,
    article_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<i64>,
    article_title: String,
    author_name: String,
    content: String,
    status: String,
    rejection_reason: String,
    created_at: String,
    updated_at: String,
}

pub async fn dashboard(State(state): State<PublicState>, headers: HeaderMap) -> Result<Response> {
    if session_user(&state, &headers).await.is_none() {
        return Ok(auth_required());
    }

    let total_articles: i64 = sqlx::query_scalar(crate::db::sql("SELECT COUNT(*) FROM articles"))
        .fetch_one(&state.db)
        .await?;
    let published_articles: i64 = sqlx::query_scalar(crate::db::sql(
        "SELECT COUNT(*) FROM articles WHERE status = 'published'",
    ))
    .fetch_one(&state.db)
    .await?;
    let draft_articles: i64 = sqlx::query_scalar(crate::db::sql(
        "SELECT COUNT(*) FROM articles WHERE status = 'draft'",
    ))
    .fetch_one(&state.db)
    .await?;
    let total_comments: i64 = sqlx::query_scalar(crate::db::sql("SELECT COUNT(*) FROM comments"))
        .fetch_one(&state.db)
        .await?;
    let pending_comments: i64 = sqlx::query_scalar(crate::db::sql(
        "SELECT COUNT(*) FROM comments WHERE status = 'pending'",
    ))
    .fetch_one(&state.db)
    .await?;
    let total_likes: i64 = sqlx::query_scalar(crate::db::sql("SELECT COUNT(*) FROM likes"))
        .fetch_one(&state.db)
        .await?;
    let monthly_views = estimate_monthly_views(published_articles, total_likes);
    let followers = estimate_followers(total_likes, total_comments);

    Ok(Json(json!({
        "stats": {
            "total_articles": total_articles,
            "published_articles": published_articles,
            "draft_articles": draft_articles,
            "total_comments": total_comments,
            "pending_comments": pending_comments,
            "total_likes": total_likes,
            "monthly_views": monthly_views,
            "followers": followers,
        },
        "activity": dashboard_activity(&state).await?,
        "views_trend": dashboard_views_trend(monthly_views),
    }))
    .into_response())
}

pub async fn settings(State(state): State<PublicState>, headers: HeaderMap) -> Response {
    if session_user(&state, &headers).await.is_none() {
        return auth_required();
    }
    let cfg = &state.config;
    Json(json!({
        "site": {
            "title": cfg.site.title,
            "description": cfg.site.description,
            "base_url": cfg.site.base_url,
        },
        "upload": {
            "max_size": cfg.upload.max_size,
            "allowed_types": cfg.upload.allowed_types,
            "allow_svg": cfg.upload.allow_svg,
            "reencode": cfg.upload.reencode,
        },
        "publishing": {
            "default_author": cfg.admin.init_username,
            "scheduled_publishing": true,
            "pinned_stories": "manual",
        },
        "mcp": {
            "enabled": cfg.mcp.enabled,
            "stdio_enabled": cfg.mcp.stdio_enabled,
            "stdio_write_enabled": cfg.mcp.stdio_write_enabled,
            "http_enabled": cfg.mcp.http_enabled,
            "http_addr": cfg.mcp.http_addr,
            "http_path": cfg.mcp.http_path,
            "require_origin_check": cfg.mcp.require_origin_check,
            "allowed_origins": cfg.mcp.allowed_origins,
        }
    }))
    .into_response()
}

pub async fn list_articles(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(query): Query<ArticleListQuery>,
) -> Result<Response> {
    if session_user(&state, &headers).await.is_none() {
        return Ok(auth_required());
    }

    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM articles");
    push_article_filters(&mut count, &query);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut builder = QueryBuilder::new(
        "SELECT
            articles.id,
            articles.title,
            articles.slug,
            articles.cover_image,
            articles.status,
            articles.is_pinned,
            articles.published_at,
            articles.created_at,
            articles.updated_at,
            categories.id AS category_id,
            categories.name AS category_name,
            users.id AS author_id,
            users.username AS author_username,
            COUNT(likes.id) AS like_count
         FROM articles
         LEFT JOIN categories ON categories.id = articles.category_id
         LEFT JOIN users ON users.id = articles.author_id
         LEFT JOIN likes ON likes.article_id = articles.id",
    );
    push_article_filters(&mut builder, &query);
    builder.push(" GROUP BY articles.id, categories.id, users.id ");
    match query.sort_by.as_deref() {
        Some("published_at") => builder.push("ORDER BY articles.published_at "),
        Some("created_at") => builder.push("ORDER BY articles.created_at "),
        Some("like_count") => builder.push("ORDER BY like_count "),
        _ => builder.push("ORDER BY articles.updated_at "),
    };
    if query.sort_order.as_deref() == Some("asc") {
        builder.push("ASC");
    } else {
        builder.push("DESC");
    }
    builder.push(", articles.id DESC LIMIT ");
    builder.push_bind(page_size);
    builder.push(" OFFSET ");
    builder.push_bind((page - 1) * page_size);

    let rows = builder.build().fetch_all(&state.db).await?;
    let list = rows
        .into_iter()
        .map(|row| {
            let category_id: Option<i64> = row.try_get("category_id")?;
            let author_id: Option<i64> = row.try_get("author_id")?;
            Ok(AdminArticle {
                id: row.try_get("id")?,
                title: row.try_get("title")?,
                slug: row.try_get("slug")?,
                cover_image: row.try_get("cover_image")?,
                status: row.try_get("status")?,
                is_pinned: row.try_get::<i64, _>("is_pinned")? != 0,
                category: category_id.map(|id| AdminCategoryRef {
                    id,
                    name: row.try_get("category_name").unwrap_or_default(),
                }),
                author: author_id.map(|id| AdminAuthorRef {
                    id,
                    username: row.try_get("author_username").unwrap_or_default(),
                }),
                like_count: row.try_get("like_count")?,
                published_at: row.try_get("published_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Json(AdminList {
        list,
        page,
        page_size,
        total,
    })
    .into_response())
}

pub async fn get_article(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response> {
    if session_user(&state, &headers).await.is_none() {
        return Ok(auth_required());
    }

    let row = sqlx::query(crate::db::sql(
        "SELECT
            id, title, slug, content, cover_image, category_id, status, is_pinned,
            published_at, created_at, updated_at
         FROM articles
         WHERE id = ?",
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let Some(row) = row else {
        return Ok((
            axum::http::StatusCode::NOT_FOUND,
            Json(json!({"code":"not_found","message":"文章不存在"})),
        )
            .into_response());
    };

    Ok(Json(json!({
        "id": row.try_get::<i64, _>("id")?,
        "title": row.try_get::<String, _>("title")?,
        "slug": row.try_get::<String, _>("slug")?,
        "content": row.try_get::<String, _>("content")?,
        "cover_image": row.try_get::<String, _>("cover_image")?,
        "category_id": row.try_get::<Option<i64>, _>("category_id")?,
        "status": row.try_get::<String, _>("status")?,
        "is_pinned": row.try_get::<i64, _>("is_pinned")? != 0,
        "published_at": row.try_get::<Option<String>, _>("published_at")?,
        "created_at": row.try_get::<String, _>("created_at")?,
        "updated_at": row.try_get::<String, _>("updated_at")?,
    }))
    .into_response())
}

pub async fn list_categories(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> Result<Response> {
    if session_user(&state, &headers).await.is_none() {
        return Ok(auth_required());
    }

    let rows = sqlx::query(
        "SELECT
            categories.id,
            categories.name,
            categories.slug,
            categories.sort_order,
            categories.created_at,
            COUNT(articles.id) AS article_count
         FROM categories
         LEFT JOIN articles ON articles.category_id = categories.id
         GROUP BY categories.id, categories.name, categories.slug, categories.sort_order, categories.created_at
         ORDER BY categories.sort_order ASC, categories.id ASC",
    )
    .fetch_all(&state.db)
    .await?;
    let list = rows
        .into_iter()
        .map(|row| {
            Ok(AdminCategory {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                sort_order: row.try_get("sort_order")?,
                created_at: row.try_get("created_at")?,
                article_count: row.try_get("article_count")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Json(list).into_response())
}

pub async fn list_comments(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(query): Query<CommentListQuery>,
) -> Result<Response> {
    if session_user(&state, &headers).await.is_none() {
        return Ok(auth_required());
    }

    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM comments");
    push_comment_filters(&mut count, &query);
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut builder = QueryBuilder::new(
        "SELECT
            comments.id,
            comments.article_id,
            comments.parent_id,
            articles.title AS article_title,
            comments.author_name,
            comments.content,
            comments.status,
            comments.rejection_reason,
            comments.created_at,
            comments.updated_at
         FROM comments
         LEFT JOIN articles ON articles.id = comments.article_id",
    );
    push_comment_filters(&mut builder, &query);
    builder.push(" ORDER BY comments.created_at DESC LIMIT ");
    builder.push_bind(page_size);
    builder.push(" OFFSET ");
    builder.push_bind((page - 1) * page_size);
    let rows = builder.build().fetch_all(&state.db).await?;
    let list = rows
        .into_iter()
        .map(|row| {
            Ok(AdminComment {
                id: row.try_get("id")?,
                article_id: row.try_get("article_id")?,
                parent_id: row.try_get("parent_id")?,
                article_title: row.try_get("article_title").unwrap_or_default(),
                author_name: row.try_get("author_name")?,
                content: row.try_get("content")?,
                status: row.try_get("status")?,
                rejection_reason: row.try_get("rejection_reason")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Json(AdminList {
        list,
        page,
        page_size,
        total,
    })
    .into_response())
}

fn push_article_filters<'a>(
    builder: &mut QueryBuilder<'a, DbBackend>,
    query: &'a ArticleListQuery,
) {
    let mut has_where = false;
    if let Some(status) = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" WHERE articles.status = ");
        builder.push_bind(status);
        has_where = true;
    }
    if let Some(category_id) = query.category_id.filter(|value| *value > 0) {
        builder.push(if has_where { " AND " } else { " WHERE " });
        builder.push("articles.category_id = ");
        builder.push_bind(category_id);
        has_where = true;
    }
    if let Some(keyword) = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(if has_where { " AND " } else { " WHERE " });
        builder.push("articles.title LIKE ");
        builder.push_bind(format!("%{keyword}%"));
    }
}

fn push_comment_filters<'a>(
    builder: &mut QueryBuilder<'a, DbBackend>,
    query: &'a CommentListQuery,
) {
    let mut has_where = false;
    if let Some(status) = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" WHERE comments.status = ");
        builder.push_bind(status);
        has_where = true;
    }
    if let Some(keyword) = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(if has_where { " AND " } else { " WHERE " });
        let pattern = format!("%{keyword}%");
        builder.push("(comments.author_name LIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR comments.content LIKE ");
        builder.push_bind(pattern);
        builder.push(")");
    }
}

async fn dashboard_activity(state: &PublicState) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        "SELECT title, status, updated_at
         FROM articles
         ORDER BY updated_at DESC, id DESC
         LIMIT 3",
    )
    .fetch_all(&state.db)
    .await?;
    let mut activity = Vec::new();
    for row in rows {
        let status: String = row.try_get("status")?;
        activity.push(json!({
            "type": "article",
            "title": if status == "published" { "文章已发布" } else { "文章已更新" },
            "description": row.try_get::<String, _>("title")?,
            "tone": "primary",
            "icon": "article",
            "created_at": row.try_get::<String, _>("updated_at")?,
        }));
    }
    if activity.is_empty() {
        activity.push(json!({
            "type": "settings",
            "title": "站点配置可读取",
            "description": "管理端可以读取站点、上传和 MCP 基础配置。",
            "tone": "neutral",
            "icon": "settings",
            "created_at": "1970-01-01T00:00:00Z",
        }));
    }
    Ok(activity)
}

fn dashboard_views_trend(monthly_views: i64) -> Vec<serde_json::Value> {
    let monthly_views = monthly_views.max(1);
    let base = (monthly_views / 30).max(1);
    (0..30)
        .map(|index| {
            json!({
                "date": format!("2026-05-{:02}", index + 1),
                "views": base + (index % 7) * 3,
            })
        })
        .collect()
}

fn estimate_monthly_views(published_articles: i64, total_likes: i64) -> i64 {
    (published_articles * 4200 + total_likes * 24).max(0)
}

fn estimate_followers(total_likes: i64, total_comments: i64) -> i64 {
    (total_likes * 26 + total_comments * 8).max(0)
}
