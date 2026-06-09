use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::{
    admin_permissions::{
        self, normalize_role, permission_definitions, replace_role_permissions, require_permission,
        require_permission_csrf, role_definitions, role_permissions, UpdateRolePermissionsRequest,
        PERMISSION_USERS,
    },
    db::DbRow,
    error::{AppError, Result},
    http_public::PublicState,
};

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    username: String,
    email: String,
    password: String,
    role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRoleRequest {
    role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    username: String,
    email: String,
    role: String,
}

const LOGIN_NAME_ERROR: &str =
    "登录名需为 3-64 位字母、数字、点、下划线或短横线，不能包含空格或中文";

#[derive(Debug, Serialize)]
struct ManagedUser {
    id: i64,
    username: String,
    email: String,
    role: String,
    article_count: i64,
    created_at: String,
    permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RelatedArticleCategory {
    id: i64,
    name: String,
    slug: String,
}

#[derive(Debug, Serialize)]
struct RelatedUserArticle {
    id: i64,
    title: String,
    slug: String,
    status: String,
    published_at: Option<String>,
    updated_at: String,
    category: Option<RelatedArticleCategory>,
}

pub async fn list_users(State(state): State<PublicState>, headers: HeaderMap) -> Result<Response> {
    if let Err(response) = require_permission(&state, &headers, PERMISSION_USERS).await {
        return Ok(response);
    }

    let rows = sqlx::query(
        "SELECT
            users.id,
            users.username,
            COALESCE(users.email, '') AS email,
            users.role,
            users.created_at,
            COUNT(articles.id) AS article_count
         FROM users
         LEFT JOIN articles ON articles.author_id = users.id
         GROUP BY users.id, users.username, users.email, users.role, users.created_at
         ORDER BY users.id ASC",
    )
    .fetch_all(&state.db)
    .await?;
    let mut list = Vec::with_capacity(rows.len());
    for row in rows {
        list.push(managed_user_from_row(&state, row).await?);
    }

    Ok(Json(json!({
        "list": list,
        "roles": role_definitions(&state).await?,
        "permissions": permission_definitions(),
    }))
    .into_response())
}

pub async fn get_user(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response> {
    if let Err(response) = require_permission(&state, &headers, PERMISSION_USERS).await {
        return Ok(response);
    }
    let Some(user) = fetch_managed_user(&state, id).await? else {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "用户不存在"));
    };
    let recent_articles = fetch_related_user_articles(&state, id).await?;

    Ok(Json(json!({
        "user": user,
        "recent_articles": recent_articles,
        "roles": role_definitions(&state).await?,
        "permissions": permission_definitions(),
    }))
    .into_response())
}

pub async fn create_user(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> Result<Response> {
    if let Err(response) = require_permission_csrf(&state, &headers, PERMISSION_USERS).await {
        return Ok(response);
    }

    let username = request.username.trim();
    if !valid_username(username) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            LOGIN_NAME_ERROR,
        ));
    }
    let email = request.email.trim().to_lowercase();
    if !valid_email(&email) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "邮箱格式不正确",
        ));
    }
    let password = request.password.trim();
    if password.chars().count() < 8 {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "密码不能少于 8 个字符",
        ));
    }
    let role = normalize_role(&request.role)?;
    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|err| AppError::Config(err.to_string()))?;

    let user_id = sqlx::query_scalar(crate::db::sql(
        "INSERT INTO users (username, password, role, email, email_verified_at, created_at)
         VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP::text, CURRENT_TIMESTAMP::text)
         RETURNING id",
    ))
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .bind(&email)
    .fetch_one(&state.db)
    .await;
    let user_id = match user_id {
        Ok(user_id) => user_id,
        Err(_) => {
            return Ok(json_error(
                StatusCode::CONFLICT,
                "conflict",
                "用户名或邮箱已存在",
            ));
        }
    };
    let Some(user) = fetch_managed_user(&state, user_id).await? else {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "用户不存在"));
    };

    Ok((StatusCode::CREATED, Json(json!({ "user": user }))).into_response())
}

pub async fn update_user(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Response> {
    let admin = match require_permission_csrf(&state, &headers, PERMISSION_USERS).await {
        Ok(admin) => admin,
        Err(response) => return Ok(response),
    };
    let username = request.username.trim();
    if !valid_username(username) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            LOGIN_NAME_ERROR,
        ));
    }
    let email = request.email.trim().to_lowercase();
    if !valid_email(&email) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "邮箱格式不正确",
        ));
    }
    let role = normalize_role(&request.role)?;
    if id == admin.id && role != "admin" {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "不能移除当前登录管理员的 admin 角色",
        ));
    }

    let result = sqlx::query(crate::db::sql(
        "UPDATE users
         SET username = ?, email = ?, role = ?
         WHERE id = ?",
    ))
    .bind(username)
    .bind(&email)
    .bind(role)
    .bind(id)
    .execute(&state.db)
    .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => {
            return Ok(json_error(
                StatusCode::CONFLICT,
                "conflict",
                "用户名或邮箱已存在",
            ));
        }
    };
    if result.rows_affected() == 0 {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "用户不存在"));
    }
    let Some(user) = fetch_managed_user(&state, id).await? else {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "用户不存在"));
    };
    let recent_articles = fetch_related_user_articles(&state, id).await?;

    Ok(Json(json!({
        "user": user,
        "recent_articles": recent_articles,
        "roles": role_definitions(&state).await?,
        "permissions": permission_definitions(),
    }))
    .into_response())
}

pub async fn update_user_role(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<UpdateUserRoleRequest>,
) -> Result<Response> {
    let admin = match require_permission_csrf(&state, &headers, PERMISSION_USERS).await {
        Ok(admin) => admin,
        Err(response) => return Ok(response),
    };
    let role = normalize_role(&request.role)?;
    if id == admin.id && role != "admin" {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "不能移除当前登录管理员的 admin 角色",
        ));
    }

    let result = sqlx::query(crate::db::sql("UPDATE users SET role = ? WHERE id = ?"))
        .bind(role)
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "用户不存在"));
    }
    let Some(user) = fetch_managed_user(&state, id).await? else {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "用户不存在"));
    };

    Ok(Json(json!({ "user": user })).into_response())
}

pub async fn delete_user(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response> {
    let admin = match require_permission_csrf(&state, &headers, PERMISSION_USERS).await {
        Ok(admin) => admin,
        Err(response) => return Ok(response),
    };
    if id == admin.id {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "不能删除当前登录管理员",
        ));
    }

    let result = sqlx::query(crate::db::sql("DELETE FROM users WHERE id = ?"))
        .bind(id)
        .execute(&state.db)
        .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => {
            return Ok(json_error(
                StatusCode::CONFLICT,
                "conflict",
                "用户仍有关联内容，无法删除",
            ));
        }
    };
    if result.rows_affected() == 0 {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "用户不存在"));
    }

    Ok(Json(json!({ "deleted": true })).into_response())
}

pub async fn update_role_permissions(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<UpdateRolePermissionsRequest>,
) -> Result<Response> {
    let admin = match require_permission_csrf(&state, &headers, PERMISSION_USERS).await {
        Ok(admin) => admin,
        Err(response) => return Ok(response),
    };
    match replace_role_permissions(&state, request, &admin).await? {
        Ok(roles) => Ok(Json(json!({
            "roles": roles,
            "permissions": permission_definitions(),
        }))
        .into_response()),
        Err(response) => Ok(response),
    }
}

async fn fetch_managed_user(state: &PublicState, id: i64) -> Result<Option<ManagedUser>> {
    let row = sqlx::query(crate::db::sql(
        "SELECT
            users.id,
            users.username,
            COALESCE(users.email, '') AS email,
            users.role,
            users.created_at,
            COUNT(articles.id) AS article_count
         FROM users
         LEFT JOIN articles ON articles.author_id = users.id
         WHERE users.id = ?
         GROUP BY users.id, users.username, users.email, users.role, users.created_at",
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    match row {
        Some(row) => Ok(Some(managed_user_from_row(state, row).await?)),
        None => Ok(None),
    }
}

async fn fetch_related_user_articles(
    state: &PublicState,
    user_id: i64,
) -> Result<Vec<RelatedUserArticle>> {
    let rows = sqlx::query(crate::db::sql(
        "SELECT
            articles.id,
            articles.title,
            articles.slug,
            articles.status,
            articles.published_at,
            articles.updated_at,
            categories.id AS category_id,
            categories.name AS category_name,
            categories.slug AS category_slug
         FROM articles
         LEFT JOIN categories ON categories.id = articles.category_id
         WHERE articles.author_id = ?
         ORDER BY articles.updated_at DESC, articles.id DESC
         LIMIT 20",
    ))
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    rows.into_iter()
        .map(|row| {
            let category_id: Option<i64> = row.try_get("category_id")?;
            Ok(RelatedUserArticle {
                id: row.try_get("id")?,
                title: row.try_get("title")?,
                slug: row.try_get("slug")?,
                status: row.try_get("status")?,
                published_at: row.try_get("published_at")?,
                updated_at: row.try_get("updated_at")?,
                category: category_id.map(|id| RelatedArticleCategory {
                    id,
                    name: row.try_get("category_name").unwrap_or_default(),
                    slug: row.try_get("category_slug").unwrap_or_default(),
                }),
            })
        })
        .collect()
}

async fn managed_user_from_row(state: &PublicState, row: DbRow) -> Result<ManagedUser> {
    let role: String = row.try_get("role")?;
    let permissions = role_permissions(state, &role).await?;
    Ok(ManagedUser {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        email: row.try_get("email")?,
        article_count: row.try_get("article_count")?,
        created_at: row.try_get("created_at")?,
        permissions,
        role,
    })
}

fn valid_username(value: &str) -> bool {
    let len = value.chars().count();
    (3..=64).contains(&len)
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.ends_with('.')
        && !value.contains(' ')
}

fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    admin_permissions::json_error(status, code, message)
}
