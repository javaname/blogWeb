use axum::{
    extract::State,
    http::{
        header::{COOKIE, SET_COOKIE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Pool, Row, Sqlite};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{error::Result, http_public::PublicState};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub(crate) id: i64,
    pub(crate) username: String,
    pub(crate) role: String,
    pub(crate) csrf_token: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    user: AdminUser,
}

#[derive(Debug, Serialize)]
struct AdminUser {
    id: i64,
    username: String,
    email: String,
    role: String,
}

pub async fn login(
    State(state): State<PublicState>,
    Json(request): Json<LoginRequest>,
) -> Result<Response> {
    let username = request.username.trim();
    let password = request.password.trim();
    if username.is_empty() || password.is_empty() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "用户名和密码不能为空",
        ));
    }

    let user = find_user(&state.db, username).await?;
    let Some(user) = user.filter(|user| user.password == password) else {
        return Ok(json_error(
            StatusCode::UNAUTHORIZED,
            "auth_failed",
            "用户名或密码错误",
        ));
    };

    let session_id = session_token();
    let csrf_token = session_token();
    let mut sessions = state
        .sessions
        .write()
        .map_err(|err| crate::error::AppError::Config(err.to_string()))?;
    sessions.insert(
        session_id.clone(),
        SessionUser {
            id: user.id,
            username: user.username.clone(),
            role: user.role.clone(),
            csrf_token,
        },
    );
    drop(sessions);

    let mut response = Json(LoginResponse {
        user: AdminUser {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
        },
    })
    .into_response();
    let cookie = format!("admin_session={session_id}; Path=/; Max-Age=86400; HttpOnly");
    response.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_str(&cookie)
            .map_err(|err| crate::error::AppError::Config(err.to_string()))?,
    );
    Ok(response)
}

pub async fn csrf_token(State(state): State<PublicState>, headers: HeaderMap) -> Response {
    let Some(user) = session_user(&state, &headers) else {
        return auth_required();
    };
    Json(json!({ "csrf_token": user.csrf_token })).into_response()
}

pub async fn current_user(State(state): State<PublicState>, headers: HeaderMap) -> Response {
    let Some(user) = session_user(&state, &headers) else {
        return auth_required();
    };
    Json(json!({
        "user": {
            "id": user.id,
            "username": user.username,
            "role": user.role,
        }
    }))
    .into_response()
}

pub fn auth_required() -> Response {
    json_error(StatusCode::UNAUTHORIZED, "auth_required", "请先登录")
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

pub fn session_user(state: &PublicState, headers: &HeaderMap) -> Option<SessionUser> {
    let cookie = headers.get(COOKIE)?.to_str().ok()?;
    let session_id = cookie.split(';').find_map(|part| {
        let trimmed = part.trim();
        trimmed
            .strip_prefix("admin_session=")
            .filter(|value| !value.is_empty())
    })?;
    let sessions = state.sessions.read().ok()?;
    sessions.get(session_id).cloned()
}

#[derive(Debug)]
struct StoredUser {
    id: i64,
    username: String,
    password: String,
    role: String,
    email: String,
}

async fn find_user(pool: &Pool<Sqlite>, username: &str) -> Result<Option<StoredUser>> {
    let row = if username.contains('@') {
        sqlx::query(
            "SELECT id, username, password, role, COALESCE(email, '') AS email
             FROM users
             WHERE LOWER(email) = LOWER(?)",
        )
        .bind(username)
        .fetch_optional(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, username, password, role, COALESCE(email, '') AS email
             FROM users
             WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(pool)
        .await?
    };

    row.map(|row| {
        Ok(StoredUser {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            password: row.try_get("password")?,
            role: row.try_get("role")?,
            email: row.try_get("email")?,
        })
    })
    .transpose()
}

fn session_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:032x}{counter:016x}")
}
