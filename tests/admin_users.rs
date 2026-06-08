use axum::body::Body;
use axum::http::{header::SET_COOKIE, Method, Request, StatusCode};
use blogweb::db;
use serde_json::{json, Value};
use sqlx::Pool;
use tower::ServiceExt;

mod support;

async fn seeded_pool() -> Pool<sqlx::Sqlite> {
    let pool = db::connect_memory().await.unwrap();
    db::apply_migrations(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password, role, email, created_at)
         VALUES
         (1, 'admin', ?, 'admin', 'admin@example.com', '2026-05-29T00:00:00Z'),
         (2, 'editor', ?, 'editor', 'editor@example.com', '2026-05-29T01:00:00Z')",
    )
    .bind(support::ADMIN_PASSWORD_HASH)
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
            1, 'Editor story', 'editor-story', '# body', '', 'body', 1, 2,
            'published', 0, '2026-05-29T08:00:00Z',
            '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'
         )",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn admin_session(router: axum::Router, username: &str) -> (String, String) {
    let login_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"{}","password":"{}"}}"#,
                    username,
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

async fn get_json(router: axum::Router, uri: &str, cookie: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let response = router
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload = serde_json::from_slice(&body).unwrap_or_else(|_| {
        json!({
            "raw": String::from_utf8_lossy(&body).to_string(),
        })
    });
    (status, payload)
}

async fn json_request(
    router: axum::Router,
    method: Method,
    uri: &str,
    cookie: &str,
    csrf: &str,
    body: &str,
) -> (StatusCode, Value) {
    let response = router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
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
    let payload = serde_json::from_slice(&body).unwrap_or_else(|_| {
        json!({
            "raw": String::from_utf8_lossy(&body).to_string(),
        })
    });
    (status, payload)
}

#[tokio::test]
async fn admin_user_routes_require_admin_role() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);

    let (missing_status, missing) = get_json(router.clone(), "/api/admin/users", None).await;
    assert_eq!(missing_status, StatusCode::UNAUTHORIZED);
    assert_eq!(missing["code"], "auth_required");

    let (editor_cookie, editor_csrf) = admin_session(router.clone(), "editor").await;
    let (list_status, list_body) =
        get_json(router.clone(), "/api/admin/users", Some(&editor_cookie)).await;
    assert_eq!(list_status, StatusCode::FORBIDDEN);
    assert_eq!(list_body["code"], "forbidden");

    let (create_status, create_body) = json_request(
        router,
        Method::POST,
        "/api/admin/users",
        &editor_cookie,
        &editor_csrf,
        r#"{"username":"writer","email":"writer@example.com","password":"admin-password","role":"writer"}"#,
    )
    .await;
    assert_eq!(create_status, StatusCode::FORBIDDEN);
    assert_eq!(create_body["code"], "forbidden");
}

#[tokio::test]
async fn admin_can_list_users_with_permissions_and_article_counts() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (cookie, _) = admin_session(router.clone(), "admin").await;

    let (status, payload) = get_json(router, "/api/admin/users", Some(&cookie)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["list"][0]["username"], "admin");
    assert_eq!(payload["list"][0]["role"], "admin");
    assert_eq!(payload["list"][0]["permissions"][0], "publish");
    assert_eq!(payload["list"][1]["username"], "editor");
    assert_eq!(payload["list"][1]["article_count"], 1);
    assert_eq!(payload["roles"][0]["key"], "admin");
    assert_eq!(payload["permissions"][0]["key"], "publish");
}

#[tokio::test]
async fn admin_can_create_update_and_delete_users() {
    let redis = support::FakeRedis::start();
    let pool = seeded_pool().await;
    let router = support::router_with_redis(pool.clone(), &redis);
    let (cookie, csrf) = admin_session(router.clone(), "admin").await;

    let (create_status, created) = json_request(
        router.clone(),
        Method::POST,
        "/api/admin/users",
        &cookie,
        &csrf,
        r#"{"username":"writer","email":"writer@example.com","password":"admin-password","role":"writer"}"#,
    )
    .await;
    assert_eq!(create_status, StatusCode::CREATED);
    assert_eq!(created["user"]["username"], "writer");
    assert_eq!(created["user"]["role"], "writer");
    assert_eq!(created["user"]["permissions"][0], "publish");
    let user_id = created["user"]["id"].as_i64().unwrap();

    let (update_status, updated) = json_request(
        router.clone(),
        Method::PUT,
        &format!("/api/admin/users/{user_id}/role"),
        &cookie,
        &csrf,
        r#"{"role":"editor"}"#,
    )
    .await;
    assert_eq!(update_status, StatusCode::OK);
    assert_eq!(updated["user"]["role"], "editor");
    assert!(updated["user"]["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "moderate"));

    let (delete_status, deleted) = json_request(
        router,
        Method::DELETE,
        &format!("/api/admin/users/{user_id}"),
        &cookie,
        &csrf,
        "{}",
    )
    .await;
    assert_eq!(delete_status, StatusCode::OK);
    assert_eq!(deleted["deleted"], true);
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(exists, 0);
}

#[tokio::test]
async fn admin_create_user_rejects_duplicate_identity() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (cookie, csrf) = admin_session(router.clone(), "admin").await;

    let (status, payload) = json_request(
        router,
        Method::POST,
        "/api/admin/users",
        &cookie,
        &csrf,
        r#"{"username":"editor","email":"editor@example.com","password":"admin-password","role":"writer"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "conflict");
}
