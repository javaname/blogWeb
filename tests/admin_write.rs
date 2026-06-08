use axum::body::Body;
use axum::http::{header::SET_COOKIE, Method, Request, StatusCode};
use blogweb::{app, db};
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
    support::reset_id_sequence(&pool, "users").await;
    support::reset_id_sequence(&pool, "categories").await;
    support::reset_id_sequence(&pool, "articles").await;
    support::reset_id_sequence(&pool, "comments").await;
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
                .body(Body::from(format!(
                    r#"{{"username":"admin","password":"{}"}}"#,
                    support::ADMIN_PASSWORD
                )))
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

async fn multipart_upload(router: axum::Router, cookie: &str, csrf: &str) -> (StatusCode, Value) {
    let boundary = "blogweb-test-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"cover.png\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(&tiny_png());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/admin/upload")
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
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

fn tiny_png() -> Vec<u8> {
    vec![
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, b'I', b'H', b'D',
        b'R', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, b'I', b'D', b'A', b'T', 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, b'I',
        b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82,
    ]
}

#[tokio::test]
async fn admin_writes_require_valid_csrf_token() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
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
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
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
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
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
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
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

#[tokio::test]
async fn admin_can_get_update_and_delete_article_with_csrf() {
    let redis = support::FakeRedis::start();
    let pool = seeded_pool().await;
    let router = support::router_with_redis(pool.clone(), &redis);
    let (cookie, token) = admin_session(router.clone()).await;

    let (detail_status, detail) = json_request(
        router.clone(),
        Method::GET,
        "/api/admin/articles/1",
        &cookie,
        Some(&token),
        "",
    )
    .await;
    assert_eq!(detail_status, StatusCode::OK);
    assert_eq!(detail["title"], "Published article");
    assert_eq!(detail["content"], "# body");

    let (update_status, updated) = json_request(
        router.clone(),
        Method::PUT,
        "/api/admin/articles/1",
        &cookie,
        Some(&token),
        r##"{"title":"Updated Rust Post","content":"# updated","category_id":null,"status":"draft","is_pinned":true,"published_at":null}"##,
    )
    .await;
    assert_eq!(update_status, StatusCode::OK);
    assert_eq!(updated["title"], "Updated Rust Post");
    assert_eq!(updated["slug"], "updated-rust-post");
    assert_eq!(updated["category_id"], Value::Null);
    assert_eq!(updated["status"], "draft");
    assert_eq!(updated["is_pinned"], true);

    let old_slug_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM slug_history WHERE old_slug = 'published-article'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(old_slug_count, 1);

    let (delete_status, delete_payload) = json_request(
        router.clone(),
        Method::DELETE,
        "/api/admin/articles/1",
        &cookie,
        Some(&token),
        "",
    )
    .await;
    assert_eq!(delete_status, StatusCode::OK);
    assert_eq!(delete_payload["message"], "删除成功");

    let (missing_status, missing_payload) = json_request(
        router,
        Method::GET,
        "/api/admin/articles/1",
        &cookie,
        Some(&token),
        "",
    )
    .await;
    assert_eq!(missing_status, StatusCode::NOT_FOUND);
    assert_eq!(missing_payload["code"], "not_found");
}

#[tokio::test]
async fn admin_can_update_sort_and_delete_categories_with_csrf() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (cookie, token) = admin_session(router.clone()).await;

    let (create_status, created) = json_request(
        router.clone(),
        Method::POST,
        "/api/admin/categories",
        &cookie,
        Some(&token),
        r#"{"name":"Design","slug":"design","sort_order":2}"#,
    )
    .await;
    assert_eq!(create_status, StatusCode::CREATED);
    let id = created["id"].as_i64().unwrap();

    let (update_status, updated) = json_request(
        router.clone(),
        Method::PUT,
        &format!("/api/admin/categories/{id}"),
        &cookie,
        Some(&token),
        r#"{"name":"Product Design","slug":"product-design","sort_order":1}"#,
    )
    .await;
    assert_eq!(update_status, StatusCode::OK);
    assert_eq!(updated["name"], "Product Design");
    assert_eq!(updated["slug"], "product-design");
    assert_eq!(updated["sort_order"], 1);

    let (sort_status, sort_payload) = json_request(
        router.clone(),
        Method::PUT,
        "/api/admin/categories/sort",
        &cookie,
        Some(&token),
        &format!(r#"{{"ids":[{id},1]}}"#),
    )
    .await;
    assert_eq!(sort_status, StatusCode::OK);
    assert_eq!(sort_payload["message"], "排序更新成功");

    let (delete_status, delete_payload) = json_request(
        router.clone(),
        Method::DELETE,
        &format!("/api/admin/categories/{id}"),
        &cookie,
        Some(&token),
        "",
    )
    .await;
    assert_eq!(delete_status, StatusCode::OK);
    assert_eq!(delete_payload["message"], "删除成功");
}

#[tokio::test]
async fn admin_can_delete_comment_update_settings_and_upload_image_with_csrf() {
    let tempdir = tempfile::tempdir().unwrap();
    let redis = support::FakeRedis::start();
    let mut config = blogweb::config::Config::default();
    config.upload.dir = tempdir.path().to_string_lossy().to_string();
    config.redis.addr = redis.addr().to_string();
    let router = app::router_with_pool_and_config(
        seeded_pool().await,
        std::path::PathBuf::from("public/assets"),
        tempdir.path().to_path_buf(),
        config,
    );
    let (cookie, token) = admin_session(router.clone()).await;

    let (delete_comment_status, delete_comment_payload) = json_request(
        router.clone(),
        Method::DELETE,
        "/api/admin/comments/1",
        &cookie,
        Some(&token),
        "",
    )
    .await;
    assert_eq!(delete_comment_status, StatusCode::OK);
    assert_eq!(delete_comment_payload["message"], "删除成功");

    let (settings_status, settings) = json_request(
        router.clone(),
        Method::PUT,
        "/api/admin/settings",
        &cookie,
        Some(&token),
        r#"{"site":{"title":"新站点","description":"新的描述","base_url":"https://example.com"}}"#,
    )
    .await;
    assert_eq!(settings_status, StatusCode::OK);
    assert_eq!(settings["site"]["title"], "新站点");
    assert_eq!(settings["site"]["base_url"], "https://example.com");

    let (upload_status, upload) = multipart_upload(router, &cookie, &token).await;
    assert_eq!(upload_status, StatusCode::OK);
    assert!(upload["url"].as_str().unwrap().starts_with("/uploads/"));
    assert!(upload["filename"].as_str().unwrap().ends_with(".png"));
}
