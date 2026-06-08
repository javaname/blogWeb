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
    admin_auth::{auth_required, session_user},
    db::DbRow,
    error::{AppError, Result},
    http_public::PublicState,
    session::SessionUser,
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

#[derive(Debug, Serialize)]
struct ManagedUser {
    id: i64,
    username: String,
    email: String,
    role: String,
    article_count: i64,
    created_at: String,
    permissions: Vec<&'static str>,
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

#[derive(Debug, Serialize)]
struct PermissionDefinition {
    key: &'static str,
    label: &'static str,
    description: &'static str,
}

#[derive(Debug, Serialize)]
struct RoleDefinition {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    permissions: Vec<&'static str>,
}

pub async fn list_users(State(state): State<PublicState>, headers: HeaderMap) -> Result<Response> {
    if let Err(response) = require_admin(&state, &headers).await {
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
    let list = rows
        .into_iter()
        .map(managed_user_from_row)
        .collect::<Result<Vec<_>>>()?;

    Ok(Json(json!({
        "list": list,
        "roles": role_definitions(),
        "permissions": permission_definitions(),
    }))
    .into_response())
}

pub async fn get_user(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response> {
    if let Err(response) = require_admin(&state, &headers).await {
        return Ok(response);
    }
    let Some(user) = fetch_managed_user(&state, id).await? else {
        return Ok(json_error(StatusCode::NOT_FOUND, "not_found", "用户不存在"));
    };
    let recent_articles = fetch_related_user_articles(&state, id).await?;

    Ok(Json(json!({
        "user": user,
        "recent_articles": recent_articles,
        "roles": role_definitions(),
        "permissions": permission_definitions(),
    }))
    .into_response())
}

pub async fn create_user(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> Result<Response> {
    if let Err(response) = require_admin_csrf(&state, &headers).await {
        return Ok(response);
    }

    let username = request.username.trim();
    if !valid_username(username) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "用户名需为 3-64 位字母、数字、点、下划线或短横线",
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
    let admin = match require_admin_csrf(&state, &headers).await {
        Ok(admin) => admin,
        Err(response) => return Ok(response),
    };
    let username = request.username.trim();
    if !valid_username(username) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "用户名需为 3-64 位字母、数字、点、下划线或短横线",
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
        "roles": role_definitions(),
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
    let admin = match require_admin_csrf(&state, &headers).await {
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
    let admin = match require_admin_csrf(&state, &headers).await {
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

async fn require_admin(
    state: &PublicState,
    headers: &HeaderMap,
) -> std::result::Result<SessionUser, Response> {
    let Some(user) = session_user(state, headers).await else {
        return Err(auth_required());
    };
    if user.role != "admin" {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "forbidden",
            "需要管理员权限",
        ));
    }
    Ok(user)
}

async fn require_admin_csrf(
    state: &PublicState,
    headers: &HeaderMap,
) -> std::result::Result<SessionUser, Response> {
    let user = require_admin(state, headers).await?;
    let token = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok());
    if token == Some(user.csrf_token.as_str()) {
        Ok(user)
    } else {
        Err(json_error(
            StatusCode::FORBIDDEN,
            "csrf_invalid",
            "CSRF token 无效",
        ))
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
    row.map(managed_user_from_row).transpose()
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

fn managed_user_from_row(row: DbRow) -> Result<ManagedUser> {
    let role: String = row.try_get("role")?;
    Ok(ManagedUser {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        email: row.try_get("email")?,
        article_count: row.try_get("article_count")?,
        created_at: row.try_get("created_at")?,
        permissions: role_permissions(&role).to_vec(),
        role,
    })
}

fn normalize_role(role: &str) -> Result<&'static str> {
    match role.trim() {
        "admin" => Ok("admin"),
        "editor" => Ok("editor"),
        "writer" => Ok("writer"),
        "user" => Ok("user"),
        _ => Err(AppError::HttpStatus(400, "invalid_params".into())),
    }
}

fn role_permissions(role: &str) -> &'static [&'static str] {
    match role {
        "admin" => &["publish", "moderate", "settings", "users", "mcp"],
        "editor" => &["publish", "moderate"],
        "writer" => &["publish"],
        _ => &[],
    }
}

fn permission_definitions() -> Vec<PermissionDefinition> {
    vec![
        PermissionDefinition {
            key: "publish",
            label: "内容发布",
            description: "创建、编辑和发布文章",
        },
        PermissionDefinition {
            key: "moderate",
            label: "评论审核",
            description: "处理评论状态和删除违规内容",
        },
        PermissionDefinition {
            key: "settings",
            label: "系统设置",
            description: "更新站点配置和运行策略",
        },
        PermissionDefinition {
            key: "users",
            label: "用户管理",
            description: "新增用户并调整成员角色",
        },
        PermissionDefinition {
            key: "mcp",
            label: "MCP 接入",
            description: "管理外部客户端和发布能力",
        },
    ]
}

fn role_definitions() -> Vec<RoleDefinition> {
    vec![
        RoleDefinition {
            key: "admin",
            label: "管理员",
            description: "拥有全部后台权限",
            permissions: role_permissions("admin").to_vec(),
        },
        RoleDefinition {
            key: "editor",
            label: "编辑",
            description: "管理内容发布和评论审核",
            permissions: role_permissions("editor").to_vec(),
        },
        RoleDefinition {
            key: "writer",
            label: "作者",
            description: "创建和维护自己的内容",
            permissions: role_permissions("writer").to_vec(),
        },
        RoleDefinition {
            key: "user",
            label: "普通用户",
            description: "仅保留前台读者身份",
            permissions: role_permissions("user").to_vec(),
        },
    ]
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
    (
        status,
        Json(json!({
            "code": code,
            "message": message,
        })),
    )
        .into_response()
}
