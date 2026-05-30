use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    admin_auth::{auth_required, session_user, SessionUser},
    error::Result,
    http_public::PublicState,
    renderer,
};

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    name: String,
    slug: Option<String>,
    sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    title: String,
    content: String,
    cover_image: Option<String>,
    category_id: Option<i64>,
    status: Option<String>,
    is_pinned: Option<bool>,
    published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCommentStatusRequest {
    status: String,
    rejection_reason: Option<String>,
}

pub async fn create_category(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers) else {
        return Ok(csrf_error(&state, &headers));
    };

    let name = request.name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "分类名称长度需为 1-40 字符",
        ));
    }
    let slug = request
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
        .unwrap_or_else(|| slugify(name));
    if !is_valid_slug(&slug) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "分类 slug 不合法",
        ));
    }
    let result = sqlx::query(
        "INSERT INTO categories (name, slug, sort_order, created_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
    )
    .bind(name)
    .bind(&slug)
    .bind(request.sort_order.unwrap_or_default())
    .execute(&state.db)
    .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => {
            return Ok(json_error(
                StatusCode::CONFLICT,
                "conflict",
                "分类名称或 slug 已存在",
            ));
        }
    };
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": result.last_insert_rowid(),
            "name": name,
            "slug": slug,
        })),
    )
        .into_response())
}

pub async fn create_article(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<CreateArticleRequest>,
) -> Result<Response> {
    let Some(user) = require_csrf(&state, &headers) else {
        return Ok(csrf_error(&state, &headers));
    };

    let title = request.title.trim();
    if title.is_empty() || title.chars().count() > 120 {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "文章标题长度需为 1-120 字符",
        ));
    }
    if request.content.trim().is_empty() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "文章内容不能为空",
        ));
    }
    let cover_image = request.cover_image.unwrap_or_default();
    if !validate_cover_image_path(&cover_image) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "cover_image 只能引用站内上传路径或 https 图片",
        ));
    }
    let status = request.status.unwrap_or_else(|| "draft".into());
    if status != "draft" && status != "published" {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "status 必须为 draft 或 published",
        ));
    }
    if let Some(category_id) = request.category_id {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE id = ?")
            .bind(category_id)
            .fetch_one(&state.db)
            .await?;
        if exists == 0 {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "分类不存在",
            ));
        }
    }

    let slug = next_unique_slug(&state, &slugify(title)).await?;
    let (_, excerpt) = renderer::render_safe_html(&request.content)?;
    let published_at = if status == "published" {
        request
            .published_at
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "CURRENT_TIMESTAMP".into())
    } else {
        request.published_at.unwrap_or_default()
    };
    let published_at_value = if published_at == "CURRENT_TIMESTAMP" || published_at.is_empty() {
        None
    } else {
        Some(published_at.as_str())
    };

    let result = if status == "published" && published_at_value.is_none() {
        sqlx::query(
            "INSERT INTO articles (
                title, slug, content, cover_image, excerpt, category_id, author_id,
                status, is_pinned, published_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(title)
        .bind(&slug)
        .bind(&request.content)
        .bind(&cover_image)
        .bind(&excerpt)
        .bind(request.category_id)
        .bind(user.id)
        .bind(&status)
        .bind(i64::from(request.is_pinned.unwrap_or(false)))
        .execute(&state.db)
        .await?
    } else {
        sqlx::query(
            "INSERT INTO articles (
                title, slug, content, cover_image, excerpt, category_id, author_id,
                status, is_pinned, published_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(title)
        .bind(&slug)
        .bind(&request.content)
        .bind(&cover_image)
        .bind(&excerpt)
        .bind(request.category_id)
        .bind(user.id)
        .bind(&status)
        .bind(i64::from(request.is_pinned.unwrap_or(false)))
        .bind(published_at_value)
        .execute(&state.db)
        .await?
    };

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": result.last_insert_rowid(),
            "slug": slug,
        })),
    )
        .into_response())
}

pub async fn update_comment_status(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<UpdateCommentStatusRequest>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers) else {
        return Ok(csrf_error(&state, &headers));
    };
    if !matches!(request.status.as_str(), "approved" | "pending" | "rejected") {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "评论状态非法",
        ));
    }
    let rejection_reason = if request.status == "rejected" {
        request
            .rejection_reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("不符合评论规范")
            .to_string()
    } else {
        String::new()
    };
    let result = sqlx::query(
        "UPDATE comments
         SET status = ?, rejection_reason = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(&request.status)
    .bind(&rejection_reason)
    .bind(id)
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "评论不存在"));
    }
    Ok(Json(json!({ "id": id, "status": request.status })).into_response())
}

fn require_csrf(state: &PublicState, headers: &HeaderMap) -> Option<SessionUser> {
    let user = session_user(state, headers)?;
    let token = headers.get("x-csrf-token")?.to_str().ok()?;
    if token == user.csrf_token {
        Some(user)
    } else {
        None
    }
}

fn csrf_error(state: &PublicState, headers: &HeaderMap) -> Response {
    if session_user(state, headers).is_none() {
        return auth_required();
    }
    json_error(StatusCode::FORBIDDEN, "csrf_invalid", "CSRF token 无效")
}

fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({
            "code": code,
            "message": message,
        })),
    )
        .into_response()
}

async fn next_unique_slug(state: &PublicState, base: &str) -> Result<String> {
    let base = if is_valid_slug(base) {
        base.to_string()
    } else {
        "article".to_string()
    };
    for index in 1..1000 {
        let candidate = if index == 1 {
            base.clone()
        } else {
            format!("{base}-{index}")
        };
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE slug = ?")
            .bind(&candidate)
            .fetch_one(&state.db)
            .await?;
        if exists == 0 {
            return Ok(candidate);
        }
    }
    Ok(format!("article-{}", chrono_like_timestamp()))
}

fn slugify(value: &str) -> String {
    let mut result = String::new();
    let mut last_dash = false;
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch);
            last_dash = false;
        } else if matches!(ch, ' ' | '-' | '_' | '.' | '/') && !last_dash && !result.is_empty() {
            result.push('-');
            last_dash = true;
        }
    }
    result.trim_matches('-').to_string()
}

fn is_valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn validate_cover_image_path(value: &str) -> bool {
    value.is_empty()
        || value.starts_with("/uploads/")
        || value.starts_with("uploads/")
        || value.starts_with("https://")
}

fn chrono_like_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
