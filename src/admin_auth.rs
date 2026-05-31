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
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};

use crate::{error::Result, http_public::PublicState, session::SessionUser};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegistrationCodeRequest {
    email: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    email: String,
    code: String,
    password: String,
    confirm_password: String,
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

    let Some(user) = find_user(&state.db, username).await? else {
        return Ok(json_error(
            StatusCode::UNAUTHORIZED,
            "auth_failed",
            "用户名或密码错误",
        ));
    };
    if !bcrypt::verify(password, &user.password).unwrap_or(false) {
        return Ok(json_error(
            StatusCode::UNAUTHORIZED,
            "auth_failed",
            "用户名或密码错误",
        ));
    }

    let (session_id, _) = state
        .session_store
        .create(user.id, user.username.clone(), user.role.clone())
        .await?;

    let mut response = Json(LoginResponse {
        user: AdminUser {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
        },
    })
    .into_response();
    let cookie = format!(
        "admin_session={session_id}; Path=/; Max-Age={}; HttpOnly",
        state.config.session.max_age
    );
    response.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_str(&cookie)
            .map_err(|err| crate::error::AppError::Config(err.to_string()))?,
    );
    Ok(response)
}

pub async fn logout(State(state): State<PublicState>, headers: HeaderMap) -> Result<Response> {
    if let Some(session_id) = session_id_from_headers(&headers) {
        state.session_store.destroy(&session_id).await?;
    }
    let mut response = Json(json!({ "message": "已退出登录" })).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_static("admin_session=; Path=/; Max-Age=-1; HttpOnly"),
    );
    Ok(response)
}

pub async fn request_registration_code(
    State(state): State<PublicState>,
    Json(request): Json<RegistrationCodeRequest>,
) -> Result<Response> {
    let email = match normalize_email(&request.email) {
        Ok(email) => email,
        Err(response) => return Ok(response),
    };
    let exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE LOWER(email) = LOWER(?)")
            .bind(&email)
            .fetch_one(&state.db)
            .await?;
    if exists > 0 {
        return Ok(json_error(StatusCode::CONFLICT, "conflict", "邮箱已注册"));
    }
    let code = registration_code(&state);
    let ttl = state.config.email.verification_ttl_sec;
    if !state.config.email.username.trim().is_empty() {
        crate::email::send_registration_code(
            &state.config.email,
            &email,
            &code,
            std::time::Duration::from_secs(ttl),
        )
        .await?;
    }
    sqlx::query(
        "INSERT INTO email_verification_codes (email, code_hash, expires_at, created_at)
         VALUES (?, ?, datetime('now', '+' || ? || ' seconds'), CURRENT_TIMESTAMP)",
    )
    .bind(&email)
    .bind(verification_code_hash(&email, &code))
    .bind(ttl as i64)
    .execute(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "sent": true,
            "expires_in": ttl,
        })),
    )
        .into_response())
}

pub async fn register_with_email(
    State(state): State<PublicState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Response> {
    let email = match normalize_email(&request.email) {
        Ok(email) => email,
        Err(response) => return Ok(response),
    };
    let code = request.code.trim();
    let password = request.password.trim();
    let confirm_password = request.confirm_password.trim();
    if code.is_empty() || password.is_empty() || confirm_password.is_empty() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "验证码和密码不能为空",
        ));
    }
    if password != confirm_password {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "两次输入的密码不一致",
        ));
    }
    if password.chars().count() < 8 {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "密码不能少于 8 个字符",
        ));
    }
    let exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE LOWER(email) = LOWER(?)")
            .bind(&email)
            .fetch_one(&state.db)
            .await?;
    if exists > 0 {
        return Ok(json_error(StatusCode::CONFLICT, "conflict", "邮箱已注册"));
    }
    let row = sqlx::query(
        "SELECT id, code_hash
         FROM email_verification_codes
         WHERE email = ? AND used_at IS NULL AND expires_at > CURRENT_TIMESTAMP
         ORDER BY created_at DESC, id DESC
         LIMIT 1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await?;
    let Some(row) = row else {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_verification_code",
            "验证码错误或已过期",
        ));
    };
    let code_hash: String = row.try_get("code_hash")?;
    if code_hash != verification_code_hash(&email, code) {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_verification_code",
            "验证码错误或已过期",
        ));
    }
    let verification_id: i64 = row.try_get("id")?;
    let username = next_registration_username(&state.db, &email).await?;
    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|err| crate::error::AppError::Config(err.to_string()))?;
    let result = sqlx::query(
        "INSERT INTO users (username, password, role, email, email_verified_at, created_at)
         VALUES (?, ?, 'user', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(&username)
    .bind(&password_hash)
    .bind(&email)
    .execute(&state.db)
    .await?;
    sqlx::query("UPDATE email_verification_codes SET used_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(verification_id)
        .execute(&state.db)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "user": {
                "id": result.last_insert_rowid(),
                "username": username,
                "email": email,
                "role": "user",
            }
        })),
    )
        .into_response())
}

pub async fn csrf_token(State(state): State<PublicState>, headers: HeaderMap) -> Response {
    let Some(user) = session_user(&state, &headers).await else {
        return auth_required();
    };
    Json(json!({ "csrf_token": user.csrf_token })).into_response()
}

pub async fn current_user(State(state): State<PublicState>, headers: HeaderMap) -> Response {
    let Some(user) = session_user(&state, &headers).await else {
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

pub async fn session_user(state: &PublicState, headers: &HeaderMap) -> Option<SessionUser> {
    let session_id = session_id_from_headers(headers)?;
    state.session_store.get(&session_id).await.ok().flatten()
}

fn session_id_from_headers(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let trimmed = part.trim();
        trimmed
            .strip_prefix("admin_session=")
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
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

fn normalize_email(value: &str) -> std::result::Result<String, Response> {
    let email = value.trim().to_lowercase();
    if email.is_empty() {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "邮箱不能为空",
        ));
    }
    if email.chars().count() > 255 || !valid_email(&email) {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "invalid_params",
            "邮箱格式不正确",
        ));
    }
    Ok(email)
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

fn registration_code(state: &PublicState) -> String {
    if state.config.email.username.trim().is_empty() {
        return "123456".into();
    }
    let value = random_u32() % 1_000_000;
    format!("{value:06}")
}

fn random_u32() -> u32 {
    use rand::RngCore;
    rand::rngs::OsRng.next_u32()
}

fn verification_code_hash(email: &str, code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(email.as_bytes());
    hasher.update(b":");
    hasher.update(code.trim().as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn next_registration_username(pool: &Pool<Sqlite>, email: &str) -> Result<String> {
    let base = username_base_from_email(email);
    for index in 0..1000 {
        let candidate = if index == 0 {
            base.clone()
        } else {
            format!("{}-{}", truncate_username_base(&base, 116), index + 1)
        };
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ?")
            .bind(&candidate)
            .fetch_one(pool)
            .await?;
        if exists == 0 {
            return Ok(candidate);
        }
    }
    Err(crate::error::AppError::HttpStatus(409, "conflict".into()))
}

fn username_base_from_email(email: &str) -> String {
    let local = email
        .split_once('@')
        .map(|(local, _)| local)
        .unwrap_or(email);
    let mut base = String::new();
    for ch in local.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            base.push(ch);
        }
    }
    let base = base.trim_matches(['-', '_', '.']).to_string();
    if base.is_empty() {
        "user".into()
    } else {
        truncate_username_base(&base, 120)
    }
}

fn truncate_username_base(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
