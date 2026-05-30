use axum::body::Body;
use axum::http::{header::SET_COOKIE, Request, StatusCode};
use blogweb::{app, db};
use serde_json::Value;
use sqlx::Pool;
use tower::ServiceExt;

async fn seeded_pool() -> Pool<sqlx::Sqlite> {
    let pool = db::connect_memory().await.unwrap();
    db::apply_migrations(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password, role, email, created_at)
         VALUES (1, 'admin', 'admin-password', 'admin', '', '2026-05-29T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

#[tokio::test]
async fn admin_csrf_token_requires_login_like_go_golden() {
    let response = app::router_with_pool(seeded_pool().await)
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
    let router = app::router_with_pool(seeded_pool().await);
    let login_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"admin-password"}"#,
                ))
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
    let response = app::router_with_pool(seeded_pool().await)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"admin-password"}"#,
                ))
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
