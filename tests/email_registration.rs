use axum::body::Body;
use axum::http::{header::SET_COOKIE, Method, Request, StatusCode};
use blogweb::db;
use serde_json::Value;
use sqlx::Pool;
use tower::ServiceExt;

mod support;

async fn seeded_pool() -> Pool<sqlx::Sqlite> {
    let pool = db::connect_memory().await.unwrap();
    db::apply_migrations(&pool).await.unwrap();
    pool
}

async fn post_json(router: axum::Router, uri: &str, body: &str) -> (StatusCode, Value) {
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn email_registration_sends_code_creates_user_and_allows_email_login() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);

    let (code_status, code_payload) = post_json(
        router.clone(),
        "/api/auth/register/code",
        r#"{"email":"Reader@Example.com"}"#,
    )
    .await;
    assert_eq!(code_status, StatusCode::CREATED);
    assert_eq!(code_payload["sent"], true);
    assert_eq!(code_payload["expires_in"], 600);

    let (register_status, register_payload) = post_json(
        router.clone(),
        "/api/auth/register",
        r#"{"email":"reader@example.com","code":"123456","password":"reader-password","confirm_password":"reader-password"}"#,
    )
    .await;
    assert_eq!(register_status, StatusCode::CREATED);
    assert_eq!(register_payload["user"]["email"], "reader@example.com");
    assert_eq!(register_payload["user"]["role"], "user");

    let login_response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"reader@example.com","password":"reader-password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    assert!(login_response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .any(|cookie| cookie.to_str().unwrap().starts_with("admin_session=")));
}

#[tokio::test]
async fn email_registration_rejects_invalid_code_and_duplicate_email() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);

    let (code_status, _) = post_json(
        router.clone(),
        "/api/auth/register/code",
        r#"{"email":"reader@example.com"}"#,
    )
    .await;
    assert_eq!(code_status, StatusCode::CREATED);

    let (bad_status, bad_payload) = post_json(
        router.clone(),
        "/api/auth/register",
        r#"{"email":"reader@example.com","code":"000000","password":"reader-password","confirm_password":"reader-password"}"#,
    )
    .await;
    assert_eq!(bad_status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_payload["code"], "invalid_verification_code");

    let (register_status, _) = post_json(
        router.clone(),
        "/api/auth/register",
        r#"{"email":"reader@example.com","code":"123456","password":"reader-password","confirm_password":"reader-password"}"#,
    )
    .await;
    assert_eq!(register_status, StatusCode::CREATED);

    let (duplicate_status, duplicate_payload) = post_json(
        router,
        "/api/auth/register/code",
        r#"{"email":"reader@example.com"}"#,
    )
    .await;
    assert_eq!(duplicate_status, StatusCode::CONFLICT);
    assert_eq!(duplicate_payload["code"], "conflict");
}

#[tokio::test]
async fn registration_code_requires_configured_smtp_for_production_delivery() {
    let redis = support::FakeRedis::start();
    let mut config = blogweb::config::Config::default();
    config.redis.addr = redis.addr().to_string();
    config.email.username = "sender@example.com".into();
    config.email.password = String::new();
    let router = blogweb::app::router_with_pool_and_config(
        seeded_pool().await,
        std::path::PathBuf::from("public/assets"),
        std::path::PathBuf::from("public/uploads"),
        config,
    );

    let (status, payload) = post_json(
        router,
        "/api/auth/register/code",
        r#"{"email":"reader@example.com"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(payload["code"], "email_unavailable");
}

#[tokio::test]
async fn registration_code_sends_smtp_message_when_email_config_is_complete() {
    let redis = support::FakeRedis::start();
    let smtp = support::FakeSmtp::start();
    let pool = seeded_pool().await;
    let mut config = blogweb::config::Config::default();
    config.redis.addr = redis.addr().to_string();
    config.email.smtp_host = smtp.host();
    config.email.smtp_port = smtp.port();
    config.email.username = "sender@example.com".into();
    config.email.password = "smtp-password".into();
    config.email.from = "blog@example.com".into();
    config.email.allow_insecure = true;
    let router = blogweb::app::router_with_pool_and_config(
        pool.clone(),
        std::path::PathBuf::from("public/assets"),
        std::path::PathBuf::from("public/uploads"),
        config,
    );

    let (status, payload) = post_json(
        router,
        "/api/auth/register/code",
        r#"{"email":"Reader@Example.com"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(payload["sent"], true);
    let messages = smtp.messages();
    assert_eq!(messages.len(), 1, "expected one SMTP message");
    assert!(
        messages[0].contains("To: reader@example.com"),
        "{}",
        messages[0]
    );
    assert!(messages[0].contains("Subject:"), "{}", messages[0]);
    assert!(
        messages[0].contains("Content-Type: text/plain; charset=utf-8"),
        "{}",
        messages[0]
    );
    let stored: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM email_verification_codes WHERE email = 'reader@example.com'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored, 1);
}
