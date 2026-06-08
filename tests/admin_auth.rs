use axum::body::Body;
use axum::http::{header::SET_COOKIE, Request, StatusCode};
use blogweb::db;
use serde_json::Value;
use tower::ServiceExt;

mod support;

async fn seeded_pool() -> db::DbPool {
    let pool = db::connect_memory().await.unwrap();
    db::apply_migrations(&pool).await.unwrap();
    sqlx::query(db::sql(
        "INSERT INTO users (id, username, password, role, email, created_at)
         VALUES (1, 'admin', ?, 'admin', '', '2026-05-29T00:00:00Z')",
    ))
    .bind(support::ADMIN_PASSWORD_HASH)
    .execute(&pool)
    .await
    .unwrap();
    pool
}

#[tokio::test]
async fn admin_csrf_token_requires_login_like_go_golden() {
    let redis = support::FakeRedis::start();
    let response = support::router_with_redis(seeded_pool().await, &redis)
        .oneshot(
            Request::builder()
                .uri("/api/admin/csrf-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload,
        serde_json::json!({"code":"auth_required","message":"请先登录"})
    );
}

#[tokio::test]
async fn admin_login_session_allows_csrf_token_and_current_user() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let login_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"admin","password":"{}"}}"#,
                    support::ADMIN_PASSWORD
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    let admin_cookie = login_response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .find(|cookie| cookie.starts_with("admin_session="))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap()
        .to_string();

    let csrf_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/admin/csrf-token")
                .header("cookie", admin_cookie.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(csrf_response.status(), StatusCode::OK);
    let csrf_body = axum::body::to_bytes(csrf_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let csrf_payload: Value = serde_json::from_slice(&csrf_body).unwrap();
    assert!(
        csrf_payload
            .get("csrf_token")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty()),
        "payload: {csrf_payload:?}"
    );

    let me_response = router
        .oneshot(
            Request::builder()
                .uri("/api/admin/me")
                .header("cookie", admin_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me_response.status(), StatusCode::OK);
    let me_body = axum::body::to_bytes(me_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let me_payload: Value = serde_json::from_slice(&me_body).unwrap();
    assert_eq!(
        me_payload,
        serde_json::json!({
            "user": {
                "id": 1,
                "role": "admin",
                "username": "admin"
            }
        })
    );
}

#[tokio::test]
async fn admin_login_matches_go_golden_body_and_cookie_contract() {
    let redis = support::FakeRedis::start();
    let response = support::router_with_redis(seeded_pool().await, &redis)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"admin","password":"{}"}}"#,
                    support::ADMIN_PASSWORD
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let cookies = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(
        cookies
            .iter()
            .any(|cookie| cookie.starts_with("admin_session=")
                && cookie.contains("Path=/")
                && cookie.contains("Max-Age=86400")
                && cookie.contains("HttpOnly")),
        "cookies: {cookies:?}"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload,
        serde_json::json!({
            "user": {
                "email": "",
                "id": 1,
                "role": "admin",
                "username": "admin"
            }
        })
    );
}

#[tokio::test]
async fn admin_login_accepts_bcrypt_hash_and_stores_session_in_redis() {
    let redis = support::FakeRedis::start();
    let response = support::router_with_redis(seeded_pool().await, &redis)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"admin","password":"{}"}}"#,
                    support::ADMIN_PASSWORD
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let admin_cookie = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .find(|cookie| cookie.starts_with("admin_session="))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap()
        .to_string();
    let session_id = admin_cookie.trim_start_matches("admin_session=");
    let raw_session = redis
        .get(&format!("session:{session_id}"))
        .expect("session should be stored in redis");
    let session: Value = serde_json::from_str(&raw_session).unwrap();
    assert_eq!(session["user_id"], 1);
    assert_eq!(session["username"], "admin");
    assert_eq!(session["role"], "admin");
    let csrf = session["csrf_token"].as_str().unwrap();
    assert!(!csrf.is_empty());
    assert_eq!(
        redis.get(&format!("csrf:{session_id}")).as_deref(),
        Some(csrf)
    );
}

#[tokio::test]
async fn admin_logout_deletes_redis_session_and_expires_cookie() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let login_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"admin","password":"{}"}}"#,
                    support::ADMIN_PASSWORD
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let admin_cookie = login_response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .find(|cookie| cookie.starts_with("admin_session="))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap()
        .to_string();
    let session_id = admin_cookie
        .trim_start_matches("admin_session=")
        .to_string();

    let logout_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/logout")
                .header("cookie", admin_cookie.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout_response.status(), StatusCode::OK);
    let cookies = logout_response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(
        cookies.iter().any(|cookie| {
            cookie.starts_with("admin_session=")
                && cookie.contains("Path=/")
                && cookie.contains("HttpOnly")
                && (cookie.contains("Max-Age=-1") || cookie.contains("Max-Age=0"))
        }),
        "cookies: {cookies:?}"
    );
    assert!(redis.get(&format!("session:{session_id}")).is_none());
    assert!(redis.get(&format!("csrf:{session_id}")).is_none());

    let csrf_response = router
        .oneshot(
            Request::builder()
                .uri("/api/admin/csrf-token")
                .header("cookie", admin_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(csrf_response.status(), StatusCode::UNAUTHORIZED);
}
