use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::path::PathBuf;

use crate::{
    admin_auth::{auth_required, session_user},
    error::Result,
    http_public::PublicState,
    renderer,
    session::SessionUser,
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

#[derive(Debug, Deserialize)]
pub struct SortCategoriesRequest {
    ids: Vec<i64>,
}

pub async fn create_category(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
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
    let category_id = sqlx::query_scalar::<_, i64>(crate::db::sql(
        "INSERT INTO categories (name, slug, sort_order, created_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP::text)
         RETURNING id",
    ))
    .bind(name)
    .bind(&slug)
    .bind(request.sort_order.unwrap_or_default())
    .fetch_one(&state.db)
    .await;
    let category_id = match category_id {
        Ok(category_id) => category_id,
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
            "id": category_id,
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
    let Some(user) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
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
        let exists: i64 = sqlx::query_scalar(crate::db::sql(
            "SELECT COUNT(*) FROM categories WHERE id = ?",
        ))
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

    let result: i64 = if status == "published" && published_at_value.is_none() {
        sqlx::query_scalar(crate::db::sql(
            "INSERT INTO articles (
                title, slug, content, cover_image, excerpt, category_id, author_id,
                status, is_pinned, published_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP::text, CURRENT_TIMESTAMP::text, CURRENT_TIMESTAMP::text)
             RETURNING id",
        ))
        .bind(title)
        .bind(&slug)
        .bind(&request.content)
        .bind(&cover_image)
        .bind(&excerpt)
        .bind(request.category_id)
        .bind(user.id)
        .bind(&status)
        .bind(i64::from(request.is_pinned.unwrap_or(false)))
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_scalar(crate::db::sql(
            "INSERT INTO articles (
                title, slug, content, cover_image, excerpt, category_id, author_id,
                status, is_pinned, published_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP::text, CURRENT_TIMESTAMP::text)
             RETURNING id",
        ))
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
        .fetch_one(&state.db)
        .await?
    };

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": result,
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
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
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
    let result = sqlx::query(crate::db::sql(
        "UPDATE comments
         SET status = ?, rejection_reason = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    ))
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

pub async fn update_article(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(raw): Json<Value>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
    };
    let Some(object) = raw.as_object() else {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "请求体格式错误",
        ));
    };
    let row = sqlx::query(crate::db::sql(
        "SELECT title, slug, content, cover_image, category_id, status, is_pinned, published_at
         FROM articles WHERE id = ?",
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let Some(row) = row else {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "文章不存在"));
    };

    let old_slug: String = row.try_get("slug")?;
    let mut title: String = row.try_get("title")?;
    let mut slug = old_slug.clone();
    let mut content: String = row.try_get("content")?;
    let mut cover_image: String = row.try_get("cover_image")?;
    let mut category_id: Option<i64> = row.try_get("category_id")?;
    let mut status: String = row.try_get("status")?;
    let mut is_pinned: bool = row.try_get::<i64, _>("is_pinned")? != 0;
    let mut published_at: Option<String> = row.try_get("published_at")?;
    let mut slug_changed = false;

    if let Some(value) = object.get("title") {
        title = string_field(value, "title 类型错误")?.trim().to_string();
        if title.is_empty() || title.chars().count() > 120 {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "文章标题长度需为 1-120 字符",
            ));
        }
        let next_slug = next_unique_slug_excluding(&state, &slugify(&title), id).await?;
        if next_slug != old_slug {
            slug = next_slug;
            slug_changed = true;
        }
    }
    if let Some(value) = object.get("content") {
        content = string_field(value, "content 类型错误")?;
        if content.trim().is_empty() {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "文章内容不能为空",
            ));
        }
    }
    if let Some(value) = object.get("cover_image") {
        cover_image = string_field(value, "cover_image 类型错误")?;
        if !validate_cover_image_path(&cover_image) {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "cover_image 只能引用站内上传路径或 https 图片",
            ));
        }
    }
    if let Some(value) = object.get("category_id") {
        if value.is_null() {
            category_id = None;
        } else {
            let next = value.as_i64().ok_or_else(|| {
                crate::error::AppError::HttpStatus(400, "category_id 类型错误".into())
            })?;
            let exists: i64 = sqlx::query_scalar(crate::db::sql(
                "SELECT COUNT(*) FROM categories WHERE id = ?",
            ))
            .bind(next)
            .fetch_one(&state.db)
            .await?;
            if exists == 0 {
                return Ok(json_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_params",
                    "分类不存在",
                ));
            }
            category_id = Some(next);
        }
    }
    if let Some(value) = object.get("status") {
        status = string_field(value, "status 类型错误")?;
        if status != "draft" && status != "published" {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "status 必须为 draft 或 published",
            ));
        }
        if status == "published" && published_at.is_none() {
            published_at = Some("CURRENT_TIMESTAMP".into());
        }
    }
    if let Some(value) = object.get("is_pinned") {
        is_pinned = value
            .as_bool()
            .ok_or_else(|| crate::error::AppError::HttpStatus(400, "is_pinned 类型错误".into()))?;
    }
    if let Some(value) = object.get("published_at") {
        if value.is_null() || value.as_str() == Some("") {
            published_at = None;
        } else {
            published_at = Some(string_field(value, "published_at 类型错误")?);
        }
    }

    let (_, excerpt) = renderer::render_safe_html(&content)?;
    if slug_changed {
        sqlx::query(crate::db::sql(
            "INSERT INTO slug_history (article_id, old_slug, created_at)
             VALUES (?, ?, CURRENT_TIMESTAMP::text)
             ON CONFLICT(old_slug) DO NOTHING",
        ))
        .bind(id)
        .bind(&old_slug)
        .execute(&state.db)
        .await?;
    }
    if published_at.as_deref() == Some("CURRENT_TIMESTAMP") {
        sqlx::query(crate::db::sql(
            "UPDATE articles SET
                title = ?, slug = ?, content = ?, cover_image = ?, excerpt = ?,
                category_id = ?, status = ?, is_pinned = ?, published_at = CURRENT_TIMESTAMP::text,
                updated_at = CURRENT_TIMESTAMP::text
             WHERE id = ?",
        ))
        .bind(&title)
        .bind(&slug)
        .bind(&content)
        .bind(&cover_image)
        .bind(&excerpt)
        .bind(category_id)
        .bind(&status)
        .bind(i64::from(is_pinned))
        .bind(id)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query(crate::db::sql(
            "UPDATE articles SET
                title = ?, slug = ?, content = ?, cover_image = ?, excerpt = ?,
                category_id = ?, status = ?, is_pinned = ?, published_at = ?,
                updated_at = CURRENT_TIMESTAMP::text
             WHERE id = ?",
        ))
        .bind(&title)
        .bind(&slug)
        .bind(&content)
        .bind(&cover_image)
        .bind(&excerpt)
        .bind(category_id)
        .bind(&status)
        .bind(i64::from(is_pinned))
        .bind(published_at.as_deref())
        .bind(id)
        .execute(&state.db)
        .await?;
    }
    Ok(Json(json!({
        "id": id,
        "title": title,
        "slug": slug,
        "content": content,
        "cover_image": cover_image,
        "category_id": category_id,
        "status": status,
        "is_pinned": is_pinned,
        "published_at": if published_at.as_deref() == Some("CURRENT_TIMESTAMP") { Value::Null } else { json!(published_at) },
    }))
    .into_response())
}

pub async fn delete_article(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
    };
    let slug: Option<String> =
        sqlx::query_scalar(crate::db::sql("SELECT slug FROM articles WHERE id = ?"))
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let Some(slug) = slug else {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "文章不存在"));
    };
    sqlx::query(crate::db::sql(
        "INSERT INTO slug_history (old_slug, created_at)
         VALUES (?, CURRENT_TIMESTAMP::text)
         ON CONFLICT(old_slug) DO NOTHING",
    ))
    .bind(&slug)
    .execute(&state.db)
    .await?;
    sqlx::query(crate::db::sql(
        "UPDATE slug_history SET article_id = NULL WHERE article_id = ?",
    ))
    .bind(id)
    .execute(&state.db)
    .await?;
    sqlx::query(crate::db::sql("DELETE FROM articles WHERE id = ?"))
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({"message":"删除成功"})).into_response())
}

pub async fn update_category(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<Value>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
    };
    let row = sqlx::query(crate::db::sql(
        "SELECT name, slug, sort_order FROM categories WHERE id = ?",
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let Some(row) = row else {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "分类不存在"));
    };
    let object = request
        .as_object()
        .ok_or_else(|| crate::error::AppError::HttpStatus(400, "请求体格式错误".into()))?;
    let mut name: String = row.try_get("name")?;
    let mut slug: String = row.try_get("slug")?;
    let mut sort_order: i64 = row.try_get("sort_order")?;
    if let Some(value) = object.get("name") {
        name = string_field(value, "name 类型错误")?.trim().to_string();
        if name.is_empty() || name.chars().count() > 40 {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "分类名称长度需为 1-40 字符",
            ));
        }
    }
    if let Some(value) = object.get("slug") {
        slug = string_field(value, "slug 类型错误")?.trim().to_lowercase();
        if slug.is_empty() {
            slug = slugify(&name);
        }
        if !is_valid_slug(&slug) {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "分类 slug 不合法",
            ));
        }
    }
    if let Some(value) = object.get("sort_order") {
        sort_order = value
            .as_i64()
            .ok_or_else(|| crate::error::AppError::HttpStatus(400, "sort_order 类型错误".into()))?;
        if sort_order < 0 {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "sort_order 不能为负数",
            ));
        }
    }
    let result = sqlx::query(crate::db::sql(
        "UPDATE categories SET name = ?, slug = ?, sort_order = ? WHERE id = ?",
    ))
    .bind(&name)
    .bind(&slug)
    .bind(sort_order)
    .bind(id)
    .execute(&state.db)
    .await;
    if result.is_err() {
        return Ok(json_error(
            StatusCode::CONFLICT,
            "conflict",
            "分类名称或 slug 已存在",
        ));
    }
    Ok(Json(json!({
        "id": id,
        "name": name,
        "slug": slug,
        "sort_order": sort_order,
    }))
    .into_response())
}

pub async fn delete_category(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
    };
    let exists: i64 = sqlx::query_scalar(crate::db::sql(
        "SELECT COUNT(*) FROM categories WHERE id = ?",
    ))
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if exists == 0 {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "分类不存在"));
    }
    let published_count: i64 = sqlx::query_scalar(crate::db::sql(
        "SELECT COUNT(*) FROM articles WHERE category_id = ? AND status = 'published'",
    ))
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if published_count > 0 {
        return Ok(json_error(
            StatusCode::CONFLICT,
            "conflict",
            "该分类下存在已发布文章，无法删除",
        ));
    }
    sqlx::query(crate::db::sql(
        "UPDATE articles SET category_id = NULL WHERE category_id = ?",
    ))
    .bind(id)
    .execute(&state.db)
    .await?;
    sqlx::query(crate::db::sql("DELETE FROM categories WHERE id = ?"))
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({"message":"删除成功"})).into_response())
}

pub async fn sort_categories(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<SortCategoriesRequest>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
    };
    if request.ids.is_empty() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "ids 不能为空",
        ));
    }
    for (index, id) in request.ids.iter().enumerate() {
        sqlx::query(crate::db::sql(
            "UPDATE categories SET sort_order = ? WHERE id = ?",
        ))
        .bind(index as i64)
        .bind(id)
        .execute(&state.db)
        .await?;
    }
    Ok(Json(json!({"message":"排序更新成功"})).into_response())
}

pub async fn delete_comment(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
    };
    let result = sqlx::query(crate::db::sql("DELETE FROM comments WHERE id = ?"))
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "评论不存在"));
    }
    Ok(Json(json!({"message":"删除成功"})).into_response())
}

pub async fn update_settings(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
    };
    let mut title = state.config.site.title.clone();
    let mut description = state.config.site.description.clone();
    let mut base_url = state.config.site.base_url.clone();
    if let Some(site) = request.get("site").and_then(Value::as_object) {
        if let Some(value) = site.get("title") {
            title = string_field(value, "title 类型错误")?.trim().to_string();
            if title.is_empty() || title.chars().count() > 80 {
                return Ok(json_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_params",
                    "站点标题长度需为 1-80 字符",
                ));
            }
        }
        if let Some(value) = site.get("description") {
            description = string_field(value, "description 类型错误")?
                .trim()
                .to_string();
            if description.chars().count() > 200 {
                return Ok(json_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_params",
                    "站点描述不能超过 200 字符",
                ));
            }
        }
        if let Some(value) = site.get("base_url") {
            base_url = string_field(value, "base_url 类型错误")?.trim().to_string();
            if !base_url.is_empty()
                && !(base_url.starts_with("http://") || base_url.starts_with("https://"))
            {
                return Ok(json_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_params",
                    "base_url 必须是有效的 http 或 https 地址",
                ));
            }
        }
    }
    Ok(Json(json!({
        "site": {
            "title": title,
            "description": description,
            "base_url": base_url,
        },
        "upload": {
            "max_size": state.config.upload.max_size,
            "allowed_types": state.config.upload.allowed_types.clone(),
            "allow_svg": state.config.upload.allow_svg,
            "reencode": state.config.upload.reencode,
        },
        "publishing": {
            "default_author": state.config.admin.init_username,
            "scheduled_publishing": true,
            "pinned_stories": "manual",
        },
        "mcp": {
            "enabled": state.config.mcp.enabled,
            "stdio_enabled": state.config.mcp.stdio_enabled,
            "stdio_write_enabled": state.config.mcp.stdio_write_enabled,
            "http_enabled": state.config.mcp.http_enabled,
            "http_addr": state.config.mcp.http_addr,
            "http_path": state.config.mcp.http_path,
            "require_origin_check": state.config.mcp.require_origin_check,
            "allowed_origins": state.config.mcp.allowed_origins.clone(),
        }
    }))
    .into_response())
}

pub async fn upload(
    State(state): State<PublicState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Response> {
    let Some(_) = require_csrf(&state, &headers).await else {
        return Ok(csrf_error(&state, &headers).await);
    };
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| crate::error::AppError::Config(err.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let data = field
            .bytes()
            .await
            .map_err(|err| crate::error::AppError::Config(err.to_string()))?;
        if data.len() as u64 > state.config.upload.max_size {
            return Ok(json_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "文件大小超过 5MB 限制",
            ));
        }
        let Some((mime_type, ext)) = detect_image_type(&data) else {
            return Ok(json_error(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                "不支持的文件类型，仅允许 jpg/png/gif/webp",
            ));
        };
        if !state
            .config
            .upload
            .allowed_types
            .iter()
            .any(|allowed| allowed == mime_type)
        {
            return Ok(json_error(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                "不支持的文件类型，仅允许 jpg/png/gif/webp",
            ));
        }
        let (year, month) = current_utc_year_month();
        let dir = PathBuf::from(&state.config.upload.dir)
            .join(format!("{year:04}"))
            .join(format!("{month:02}"));
        tokio::fs::create_dir_all(&dir).await?;
        let filename = format!("{}{}", upload_token(), ext);
        tokio::fs::write(dir.join(&filename), &data).await?;
        return Ok(Json(json!({
            "url": format!("/uploads/{year:04}/{month:02}/{filename}"),
            "filename": filename,
            "mime_type": mime_type,
            "size": data.len(),
        }))
        .into_response());
    }
    Ok(json_error(
        StatusCode::BAD_REQUEST,
        "invalid_params",
        "缺少 file 文件",
    ))
}

async fn require_csrf(state: &PublicState, headers: &HeaderMap) -> Option<SessionUser> {
    let user = session_user(state, headers).await?;
    let token = headers.get("x-csrf-token")?.to_str().ok()?;
    if token == user.csrf_token {
        Some(user)
    } else {
        None
    }
}

async fn csrf_error(state: &PublicState, headers: &HeaderMap) -> Response {
    if session_user(state, headers).await.is_none() {
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
        let exists: i64 = sqlx::query_scalar(crate::db::sql(
            "SELECT COUNT(*) FROM articles WHERE slug = ?",
        ))
        .bind(&candidate)
        .fetch_one(&state.db)
        .await?;
        if exists == 0 {
            return Ok(candidate);
        }
    }
    Ok(format!("article-{}", chrono_like_timestamp()))
}

async fn next_unique_slug_excluding(
    state: &PublicState,
    base: &str,
    excluded_id: i64,
) -> Result<String> {
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
        let exists: i64 = sqlx::query_scalar(crate::db::sql(
            "SELECT COUNT(*) FROM articles WHERE slug = ? AND id <> ?",
        ))
        .bind(&candidate)
        .bind(excluded_id)
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

fn string_field(value: &Value, message: &str) -> Result<String> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| crate::error::AppError::HttpStatus(400, message.into()))
}

fn detect_image_type(data: &[u8]) -> Option<(&'static str, &'static str)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", ".png"))
    } else if data.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("image/jpeg", ".jpg"))
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some(("image/gif", ".gif"))
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some(("image/webp", ".webp"))
    } else {
        None
    }
}

fn upload_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn current_utc_year_month() -> (i32, u32) {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() / 86_400)
        .unwrap_or_default() as i64;
    civil_from_days(days)
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32)
}
