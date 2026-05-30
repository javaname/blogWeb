use axum::{
    extract::{Path, State},
    http::{header::COOKIE, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::{
    error::{AppError, Result},
    http_public::PublicState,
};

#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    action: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchLikesRequest {
    article_slugs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewsletterRequest {
    email: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    author_name: Option<String>,
    content: String,
    parent_id: Option<i64>,
}

pub async fn like_article(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(request): Json<ActionRequest>,
) -> Result<Response> {
    let Some(reader_id) = anonymous_id(&headers) else {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "缺少匿名访客标识",
        ));
    };
    let article_id = published_article_id(&state, &slug).await?;
    match request.action.trim() {
        "like" => {
            let result = sqlx::query(
                "INSERT OR IGNORE INTO likes (article_id, anonymous_id, ip_address, user_agent, created_at)
                 VALUES (?, ?, '', '', CURRENT_TIMESTAMP)",
            )
            .bind(article_id)
            .bind(&reader_id)
            .execute(&state.db)
            .await?;
            if result.rows_affected() == 0 {
                return Ok(json_error(StatusCode::CONFLICT, "conflict", "已经点过赞了"));
            }
        }
        "unlike" => {
            let result = sqlx::query("DELETE FROM likes WHERE article_id = ? AND anonymous_id = ?")
                .bind(article_id)
                .bind(&reader_id)
                .execute(&state.db)
                .await?;
            if result.rows_affected() == 0 {
                return Ok(json_error(
                    StatusCode::CONFLICT,
                    "conflict",
                    "尚未点赞，无法取消",
                ));
            }
        }
        _ => {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "无效的操作，action 必须为 like 或 unlike",
            ));
        }
    }
    let count = count_by_article(&state, "likes", article_id).await?;
    Ok(Json(json!({
        "liked": request.action.trim() == "like",
        "like_count": count,
    }))
    .into_response())
}

pub async fn bookmark_article(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(request): Json<ActionRequest>,
) -> Result<Response> {
    let Some(reader_id) = anonymous_id(&headers) else {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "缺少匿名访客标识",
        ));
    };
    let article_id = published_article_id(&state, &slug).await?;
    let bookmarked = match request.action.trim() {
        "bookmark" => {
            sqlx::query(
                "INSERT OR IGNORE INTO bookmarks (article_id, anonymous_id, ip_address, user_agent, created_at)
                 VALUES (?, ?, '', '', CURRENT_TIMESTAMP)",
            )
            .bind(article_id)
            .bind(&reader_id)
            .execute(&state.db)
            .await?;
            true
        }
        "unbookmark" => {
            sqlx::query("DELETE FROM bookmarks WHERE article_id = ? AND anonymous_id = ?")
                .bind(article_id)
                .bind(&reader_id)
                .execute(&state.db)
                .await?;
            false
        }
        _ => {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "无效的操作，action 必须为 bookmark 或 unbookmark",
            ));
        }
    };
    let count = count_by_article(&state, "bookmarks", article_id).await?;
    Ok(Json(json!({
        "bookmarked": bookmarked,
        "bookmark_count": count,
    }))
    .into_response())
}

pub async fn follow_author(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(author_id): Path<i64>,
    Json(request): Json<ActionRequest>,
) -> Result<Response> {
    let Some(reader_id) = anonymous_id(&headers) else {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "缺少匿名访客标识",
        ));
    };
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(author_id)
        .fetch_one(&state.db)
        .await?;
    if exists == 0 {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "作者不存在"));
    }
    let following = match request.action.trim() {
        "follow" => {
            sqlx::query(
                "INSERT OR IGNORE INTO author_follows (author_id, anonymous_id, ip_address, user_agent, created_at)
                 VALUES (?, ?, '', '', CURRENT_TIMESTAMP)",
            )
            .bind(author_id)
            .bind(&reader_id)
            .execute(&state.db)
            .await?;
            true
        }
        "unfollow" => {
            sqlx::query("DELETE FROM author_follows WHERE author_id = ? AND anonymous_id = ?")
                .bind(author_id)
                .bind(&reader_id)
                .execute(&state.db)
                .await?;
            false
        }
        _ => {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "无效的操作，action 必须为 follow 或 unfollow",
            ));
        }
    };
    let follower_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM author_follows WHERE author_id = ?")
            .bind(author_id)
            .fetch_one(&state.db)
            .await?;
    Ok(Json(json!({
        "following": following,
        "follower_count": follower_count,
    }))
    .into_response())
}

pub async fn subscribe_newsletter(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<NewsletterRequest>,
) -> Result<Response> {
    let email = request.email.trim().to_lowercase();
    if !valid_email(&email) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            if email.is_empty() {
                "邮箱不能为空"
            } else {
                "邮箱格式不正确"
            },
        ));
    }
    sqlx::query(
        "INSERT INTO newsletter_subscriptions (
            email, anonymous_id, status, ip_address, user_agent, created_at, updated_at
         ) VALUES (?, ?, 'subscribed', '', '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
         ON CONFLICT(email) DO UPDATE SET
            anonymous_id = excluded.anonymous_id,
            status = 'subscribed',
            updated_at = CURRENT_TIMESTAMP",
    )
    .bind(&email)
    .bind(anonymous_id(&headers).unwrap_or_default())
    .execute(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "subscribed": true,
            "email": email,
        })),
    )
        .into_response())
}

pub async fn create_comment(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(request): Json<CreateCommentRequest>,
) -> Result<Response> {
    let reader_id = anonymous_id(&headers).unwrap_or_default();
    let article_id = published_article_id(&state, &slug).await?;
    let author_name = request
        .author_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("匿名读者");
    if author_name.chars().count() > 40 {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "昵称不能超过 40 个字符",
        ));
    }
    let content = request.content.trim();
    if content.is_empty() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "评论内容不能为空",
        ));
    }
    if content.chars().count() > 500 {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "评论内容不能超过 500 个字符",
        ));
    }
    if let Some(parent_id) = request.parent_id {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM comments
             WHERE id = ? AND article_id = ? AND status = 'approved' AND parent_id IS NULL",
        )
        .bind(parent_id)
        .bind(article_id)
        .fetch_one(&state.db)
        .await?;
        if exists == 0 {
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "invalid_params",
                "回复的评论不存在",
            ));
        }
    }
    let result = sqlx::query(
        "INSERT INTO comments (
            article_id, parent_id, author_name, content, status, anonymous_id,
            ip_address, user_agent, created_at, updated_at
         ) VALUES (?, ?, ?, ?, 'approved', ?, '', '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(article_id)
    .bind(request.parent_id)
    .bind(author_name)
    .bind(content)
    .bind(reader_id)
    .execute(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": result.last_insert_rowid(),
            "parent_id": request.parent_id,
            "status": "approved",
            "message": "评论已发布",
        })),
    )
        .into_response())
}

pub async fn batch_likes(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<BatchLikesRequest>,
) -> Result<Response> {
    let Some(reader_id) = anonymous_id(&headers) else {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "缺少匿名访客标识",
        ));
    };
    if request.article_slugs.len() > 100 {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "article_slugs 数量不能超过 100",
        ));
    }
    let mut liked_map = serde_json::Map::new();
    for slug in request.article_slugs {
        let liked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM likes
             INNER JOIN articles ON articles.id = likes.article_id
             WHERE likes.anonymous_id = ? AND articles.slug = ?",
        )
        .bind(&reader_id)
        .bind(&slug)
        .fetch_one(&state.db)
        .await?;
        if liked > 0 {
            liked_map.insert(slug, json!(true));
        }
    }
    Ok(Json(json!({ "liked_map": liked_map })).into_response())
}

async fn published_article_id(state: &PublicState, slug: &str) -> Result<i64> {
    let row = sqlx::query(
        "SELECT id
         FROM articles
         WHERE slug = ?
           AND status = 'published'
           AND (published_at IS NULL OR published_at <= datetime('now'))",
    )
    .bind(slug)
    .fetch_optional(&state.db)
    .await?;
    let Some(row) = row else {
        return Err(AppError::HttpStatus(404, "not_found".into()));
    };
    Ok(row.try_get("id")?)
}

async fn count_by_article(state: &PublicState, table: &str, article_id: i64) -> Result<i64> {
    let sql = match table {
        "likes" => "SELECT COUNT(*) FROM likes WHERE article_id = ?",
        "bookmarks" => "SELECT COUNT(*) FROM bookmarks WHERE article_id = ?",
        _ => unreachable!(),
    };
    Ok(sqlx::query_scalar(sql)
        .bind(article_id)
        .fetch_one(&state.db)
        .await?)
}

fn anonymous_id(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers
        .get("x-anonymous-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(value.to_string());
    }
    headers
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie| {
            cookie.split(';').find_map(|part| {
                let trimmed = part.trim();
                trimmed
                    .strip_prefix("anonymous_id=")
                    .filter(|value| !value.is_empty())
            })
        })
        .map(str::to_string)
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

fn valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}
