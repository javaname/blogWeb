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
    if let Some(response) = ensure_rate_limit(
        &state,
        &rate_key("like_rate", &client_ip(&headers)),
        state.config.rate_limit.like_ip_max_requests,
        state.config.rate_limit.like_ip_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    if let Some(response) = ensure_rate_limit(
        &state,
        &article_rate_key("like_article_rate", &client_ip(&headers), article_id),
        state.config.rate_limit.like_article_max_actions,
        state.config.rate_limit.like_article_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    match request.action.trim() {
        "like" => {
            let result = sqlx::query(crate::db::sql(
                "INSERT INTO likes (article_id, anonymous_id, ip_address, user_agent, created_at)
                 VALUES (?, ?, '', '', CURRENT_TIMESTAMP::text)
                 ON CONFLICT(article_id, anonymous_id) DO NOTHING",
            ))
            .bind(article_id)
            .bind(&reader_id)
            .execute(&state.db)
            .await?;
            if result.rows_affected() == 0 {
                return Ok(json_error(StatusCode::CONFLICT, "conflict", "已经点过赞了"));
            }
        }
        "unlike" => {
            let result = sqlx::query(crate::db::sql(
                "DELETE FROM likes WHERE article_id = ? AND anonymous_id = ?",
            ))
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
    if let Some(response) = ensure_rate_limit(
        &state,
        &rate_key("bookmark_rate", &client_ip(&headers)),
        state.config.rate_limit.like_ip_max_requests,
        state.config.rate_limit.like_ip_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    if let Some(response) = ensure_rate_limit(
        &state,
        &article_rate_key("bookmark_article_rate", &client_ip(&headers), article_id),
        state.config.rate_limit.like_article_max_actions,
        state.config.rate_limit.like_article_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    let bookmarked = match request.action.trim() {
        "bookmark" => {
            sqlx::query(crate::db::sql(
                "INSERT INTO bookmarks (article_id, anonymous_id, ip_address, user_agent, created_at)
                 VALUES (?, ?, '', '', CURRENT_TIMESTAMP::text)
                 ON CONFLICT(article_id, anonymous_id) DO NOTHING",
            ))
            .bind(article_id)
            .bind(&reader_id)
            .execute(&state.db)
            .await?;
            true
        }
        "unbookmark" => {
            sqlx::query(crate::db::sql(
                "DELETE FROM bookmarks WHERE article_id = ? AND anonymous_id = ?",
            ))
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
    let exists: i64 = sqlx::query_scalar(crate::db::sql("SELECT COUNT(*) FROM users WHERE id = ?"))
        .bind(author_id)
        .fetch_one(&state.db)
        .await?;
    if exists == 0 {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "作者不存在"));
    }
    if let Some(response) = ensure_rate_limit(
        &state,
        &rate_key("follow_rate", &client_ip(&headers)),
        state.config.rate_limit.like_ip_max_requests,
        state.config.rate_limit.like_ip_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    if let Some(response) = ensure_rate_limit(
        &state,
        &article_rate_key("follow_author_rate", &client_ip(&headers), author_id),
        state.config.rate_limit.like_article_max_actions,
        state.config.rate_limit.like_article_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    let following = match request.action.trim() {
        "follow" => {
            sqlx::query(crate::db::sql(
                "INSERT INTO author_follows (author_id, anonymous_id, ip_address, user_agent, created_at)
                 VALUES (?, ?, '', '', CURRENT_TIMESTAMP::text)
                 ON CONFLICT(author_id, anonymous_id) DO NOTHING",
            ))
            .bind(author_id)
            .bind(&reader_id)
            .execute(&state.db)
            .await?;
            true
        }
        "unfollow" => {
            sqlx::query(crate::db::sql(
                "DELETE FROM author_follows WHERE author_id = ? AND anonymous_id = ?",
            ))
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
    let follower_count: i64 = sqlx::query_scalar(crate::db::sql(
        "SELECT COUNT(*) FROM author_follows WHERE author_id = ?",
    ))
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
    if let Some(response) = ensure_rate_limit(
        &state,
        &rate_key("newsletter_rate", &client_ip(&headers)),
        state.config.rate_limit.like_ip_max_requests,
        state.config.rate_limit.like_ip_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    sqlx::query(crate::db::sql(
        "INSERT INTO newsletter_subscriptions (
            email, anonymous_id, status, ip_address, user_agent, created_at, updated_at
         ) VALUES (?, ?, 'subscribed', '', '', CURRENT_TIMESTAMP::text, CURRENT_TIMESTAMP::text)
         ON CONFLICT(email) DO UPDATE SET
            anonymous_id = excluded.anonymous_id,
            status = 'subscribed',
            updated_at = CURRENT_TIMESTAMP::text",
    ))
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
    if let Some(response) = ensure_rate_limit(
        &state,
        &rate_key("comment_rate", &client_ip(&headers)),
        state.config.rate_limit.comment_ip_max_requests,
        state.config.rate_limit.comment_ip_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    if let Some(response) = ensure_rate_limit(
        &state,
        &article_rate_key("comment_article_rate", &client_ip(&headers), article_id),
        state.config.rate_limit.comment_article_max_actions,
        state.config.rate_limit.comment_article_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
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
    if let Some(message) = comment_policy_violation(author_name) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "comment_policy_violation",
            message,
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
    if let Some(message) = comment_policy_violation(content) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "comment_policy_violation",
            message,
        ));
    }
    if let Some(parent_id) = request.parent_id {
        let exists: i64 = sqlx::query_scalar(crate::db::sql(
            "SELECT COUNT(*) FROM comments
             WHERE id = ? AND article_id = ? AND status = 'approved' AND parent_id IS NULL",
        ))
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
    let comment_id: i64 = sqlx::query_scalar(crate::db::sql(
        "INSERT INTO comments (
            article_id, parent_id, author_name, content, status, anonymous_id,
            ip_address, user_agent, created_at, updated_at
         ) VALUES (?, ?, ?, ?, 'approved', ?, '', '', CURRENT_TIMESTAMP::text, CURRENT_TIMESTAMP::text)
         RETURNING id",
    ))
    .bind(article_id)
    .bind(request.parent_id)
    .bind(author_name)
    .bind(content)
    .bind(reader_id)
    .fetch_one(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": comment_id,
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
    if let Some(response) = ensure_rate_limit(
        &state,
        &rate_key("like_rate", &client_ip(&headers)),
        state.config.rate_limit.like_ip_max_requests,
        state.config.rate_limit.like_ip_window_sec,
    )
    .await?
    {
        return Ok(response);
    }
    let mut liked_map = serde_json::Map::new();
    for slug in request.article_slugs {
        let liked: i64 = sqlx::query_scalar(crate::db::sql(
            "SELECT COUNT(*)
             FROM likes
             INNER JOIN articles ON articles.id = likes.article_id
             WHERE likes.anonymous_id = ? AND articles.slug = ?",
        ))
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
    let row = sqlx::query(crate::db::sql(
        "SELECT id
         FROM articles
         WHERE slug = ?
           AND status = 'published'
           AND (published_at IS NULL OR published_at::timestamp <= CURRENT_TIMESTAMP)",
    ))
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
        "likes" => "SELECT COUNT(*) FROM likes WHERE article_id = $1",
        "bookmarks" => "SELECT COUNT(*) FROM bookmarks WHERE article_id = $1",
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

async fn ensure_rate_limit(
    state: &PublicState,
    key: &str,
    max_attempts: i64,
    window_sec: i64,
) -> Result<Option<Response>> {
    if state
        .session_store
        .allow_rate_limit(key, max_attempts, window_sec)
        .await?
    {
        Ok(None)
    } else {
        Ok(Some(json_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "请求过于频繁，请稍后再试",
        )))
    }
}

fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn rate_key(prefix: &str, ip: &str) -> String {
    format!("{prefix}:{ip}")
}

fn article_rate_key(prefix: &str, ip: &str, id: i64) -> String {
    format!("{prefix}:{ip}:{id}")
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

fn comment_policy_violation(content: &str) -> Option<&'static str> {
    let normalized = normalize_policy_text(content);
    if normalized.is_empty() {
        return None;
    }
    for rule in COMMENT_POLICY_RULES {
        if rule
            .keywords
            .iter()
            .any(|keyword| normalized.contains(&normalize_policy_text(keyword)))
        {
            return Some(rule.message);
        }
    }
    None
}

fn normalize_policy_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|value| value.is_alphanumeric())
        .collect()
}

struct CommentPolicyRule {
    message: &'static str,
    keywords: &'static [&'static str],
}

const COMMENT_POLICY_RULES: &[CommentPolicyRule] = &[
    CommentPolicyRule {
        message: "评论包含政治相关敏感内容，请修改后再提交",
        keywords: &[
            "政治",
            "政府",
            "选举",
            "总统",
            "国家主席",
            "政党",
            "议会",
            "国会",
            "外交",
            "制裁",
            "游行",
            "抗议",
            "革命",
            "分裂",
            "独立运动",
            "台独",
            "港独",
            "藏独",
            "疆独",
            "共产党",
            "国民党",
            "习近平",
            "拜登",
            "特朗普",
            "普京",
            "politics",
            "government",
            "election",
            "president",
            "congress",
            "parliament",
            "revolution",
            "protest",
        ],
    },
    CommentPolicyRule {
        message: "评论包含暴力相关敏感内容，请修改后再提交",
        keywords: &[
            "暴力",
            "杀人",
            "杀害",
            "砍人",
            "枪击",
            "枪支",
            "炸弹",
            "爆炸",
            "袭击",
            "恐怖袭击",
            "暗杀",
            "绑架",
            "虐待",
            "伤害",
            "打死",
            "打砸",
            "纵火",
            "武器",
            "violence",
            "kill",
            "murder",
            "gun",
            "bomb",
            "explosion",
            "attack",
            "weapon",
        ],
    },
    CommentPolicyRule {
        message: "评论包含血腥相关敏感内容，请修改后再提交",
        keywords: &[
            "血腥",
            "鲜血",
            "流血",
            "尸体",
            "尸块",
            "肢解",
            "断肢",
            "内脏",
            "屠杀",
            "惨死",
            "血肉模糊",
            "gore",
            "blood",
            "corpse",
            "dismember",
            "slaughter",
        ],
    },
];
