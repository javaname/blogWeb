use axum::body::Body;
use axum::http::{header::SET_COOKIE, Method, Request, StatusCode};
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
    sqlx::query(
        "INSERT INTO categories (id, name, slug, sort_order, created_at)
         VALUES (1, 'Technology', 'technology', 0, '2026-05-29T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO articles (
            id, title, slug, content, cover_image, excerpt, category_id, author_id,
            status, is_pinned, published_at, created_at, updated_at
         ) VALUES (
            1, 'Published article', 'published-article', '# body', '', 'body', 1, 1,
            'published', 0, '2026-05-29T08:00:00Z',
            '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'
         )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO comments (
            id, article_id, author_name, content, status, anonymous_id, ip_address,
            user_agent, created_at, updated_at
         ) VALUES (
            1, 1, '读者', '这是一条需要复核的评论。', 'approved', 'reader-1',
            '127.0.0.1', '', '2026-05-29T04:00:00Z', '2026-05-29T04:00:00Z'
         )",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn admin_session(router: axum::Router) -> (String, String) {
    let login_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"admin-password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login_response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .find(|cookie| cookie.starts_with("admin_session="))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap()
        .to_string();

    let csrf_response = router
        .oneshot(
            Request::builder()
                .uri("/api/admin/csrf-token")
                .header("cookie", cookie.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(csrf_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let token = payload["csrf_token"].as_str().unwrap().to_string();
    (cookie, token)
}

async fn json_request(
    router: axum::Router,
    method: Method,
    uri: &str,
    cookie: &str,
    csrf: Option<&str>,
    body: &str,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", cookie)
        .header("content-type", "application/json");
    if let Some(csrf) = csrf {
        builder = builder.header("x-csrf-token", csrf);
    }
    let response = router
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn admin_writes_require_valid_csrf_token() {
    let router = app::router_with_pool(seeded_pool().await);
    let (cookie, _token) = admin_session(router.clone()).await;

    let (status, payload) = json_request(
        router,
        Method::POST,
        "/api/admin/categories",
        &cookie,
        None,
        r#"{"name":"Design","slug":"design"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        payload,
        serde_json::json!({"code":"csrf_invalid","message":"CSRF token 无效"})
    );
}

#[tokio::test]
async fn admin_can_create_category_with_csrf() {
    let router = app::router_with_pool(seeded_pool().await);
    let (cookie, token) = admin_session(router.clone()).await;

    let (status, payload) = json_request(
        router.clone(),
        Method::POST,
        "/api/admin/categories",
        &cookie,
        Some(&token),
        r#"{"name":"Design","slug":"design","sort_order":2}"#,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(payload["name"], "Design");
    let (list_status, list_payload) = json_request(
        router,
        Method::GET,
        "/api/admin/categories",
        &cookie,
        Some(&token),
        "",
    )
    .await;
    assert_eq!(list_status, StatusCode::OK);
    assert!(list_payload
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["slug"] == "design"));
}

#[tokio::test]
async fn admin_create_article_rejects_external_http_cover_image() {
    let router = app::router_with_pool(seeded_pool().await);
    let (cookie, token) = admin_session(router.clone()).await;

    let (status, payload) = json_request(
        router,
        Method::POST,
        "/api/admin/articles",
        &cookie,
        Some(&token),
        r##"{"title":"A","content":"# body","status":"draft","cover_image":"http://evil.example/x.jpg"}"##,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["code"], "invalid_params");
}

#[tokio::test]
async fn admin_can_create_article_and_update_comment_status_with_csrf() {
    let router = app::router_with_pool(seeded_pool().await);
    let (cookie, token) = admin_session(router.clone()).await;

    let (article_status, article) = json_request(
        router.clone(),
        Method::POST,
        "/api/admin/articles",
        &cookie,
        Some(&token),
        r##"{"title":"New Rust Post","content":"# body","category_id":1,"status":"draft","cover_image":""}"##,
    )
    .await;
    assert_eq!(article_status, StatusCode::CREATED);
    assert_eq!(article["slug"], "new-rust-post");

    let (comment_status, comment) = json_request(
        router.clone(),
        Method::PUT,
        "/api/admin/comments/1/status",
        &cookie,
        Some(&token),
        r#"{"status":"rejected","rejection_reason":"不符合评论规范"}"#,
    )
    .await;
    assert_eq!(comment_status, StatusCode::OK);
    assert_eq!(comment["status"], "rejected");

    let (comments_status, comments) = json_request(
        router,
        Method::GET,
        "/api/admin/comments",
        &cookie,
        Some(&token),
        "",
    )
    .await;
    assert_eq!(comments_status, StatusCode::OK);
    assert_eq!(comments["list"][0]["status"], "rejected");
}
