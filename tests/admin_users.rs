use axum::body::Body;
use axum::http::{header::SET_COOKIE, Method, Request, StatusCode};
use blogweb::db;
use serde_json::{json, Value};
use tower::ServiceExt;

mod support;

async fn seeded_pool() -> db::DbPool {
    let pool = db::connect_memory().await.unwrap();
    db::apply_migrations(&pool).await.unwrap();
    sqlx::query(db::sql(
        "INSERT INTO users (id, username, password, role, email, created_at)
         VALUES
         (1, 'admin', ?, 'admin', 'admin@example.com', '2026-05-29T00:00:00Z'),
         (2, 'editor', ?, 'editor', 'editor@example.com', '2026-05-29T01:00:00Z')",
    ))
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
    support::reset_id_sequence(&pool, "users").await;
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
async fn current_user_includes_role_permissions_and_allowed_menus() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (editor_cookie, _) = admin_session(router.clone(), "editor").await;

    let (status, payload) = get_json(router, "/api/admin/me", Some(&editor_cookie)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["user"]["role"], "editor");
    assert!(payload["user"]["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "publish"));
    assert!(payload["user"]["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "moderate"));
    assert!(payload["user"]["menus"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["path"] == "/posts"));
    assert!(payload["user"]["menus"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["path"] == "/comments"));
    assert!(!payload["user"]["menus"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["path"] == "/users"));
}

#[tokio::test]
async fn admin_can_update_role_permissions_and_existing_users_recalculate() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (admin_cookie, admin_csrf) = admin_session(router.clone(), "admin").await;

    let (status, payload) = json_request(
        router.clone(),
        Method::PUT,
        "/api/admin/role-permissions",
        &admin_cookie,
        &admin_csrf,
        r#"{"roles":[
            {"key":"admin","permissions":["publish","moderate","settings","users","mcp","media","analytics"]},
            {"key":"editor","permissions":["publish","moderate","users","analytics"]},
            {"key":"writer","permissions":["publish"]},
            {"key":"user","permissions":[]}
        ]}"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let editor_role = payload["roles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|role| role["key"] == "editor")
        .unwrap();
    assert!(editor_role["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "users"));
    assert!(editor_role["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "analytics"));

    let (editor_cookie, _) = admin_session(router.clone(), "editor").await;
    let (me_status, me) = get_json(router.clone(), "/api/admin/me", Some(&editor_cookie)).await;
    assert_eq!(me_status, StatusCode::OK);
    assert!(me["user"]["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "users"));
    assert!(me["user"]["menus"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["path"] == "/users"));
    assert!(me["user"]["menus"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["path"] == "/roles"));
    assert!(me["user"]["menus"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["path"] == "/analytics"));

    let (users_status, users) = get_json(router, "/api/admin/users", Some(&editor_cookie)).await;
    assert_eq!(users_status, StatusCode::OK);
    assert!(users["list"][1]["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "users"));
}

#[tokio::test]
async fn admin_cannot_remove_users_permission_from_own_role() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (admin_cookie, admin_csrf) = admin_session(router.clone(), "admin").await;

    let (status, payload) = json_request(
        router,
        Method::PUT,
        "/api/admin/role-permissions",
        &admin_cookie,
        &admin_csrf,
        r#"{"roles":[
            {"key":"admin","permissions":["publish","moderate","settings","mcp"]},
            {"key":"editor","permissions":["publish","moderate"]},
            {"key":"writer","permissions":["publish"]},
            {"key":"user","permissions":[]}
        ]}"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["code"], "invalid_params");
}

#[tokio::test]
async fn backend_admin_routes_enforce_role_permissions() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (editor_cookie, _) = admin_session(router.clone(), "editor").await;

    let (settings_status, settings) =
        get_json(router.clone(), "/api/admin/settings", Some(&editor_cookie)).await;
    assert_eq!(settings_status, StatusCode::FORBIDDEN);
    assert_eq!(settings["code"], "forbidden");

    let (articles_status, _) =
        get_json(router.clone(), "/api/admin/articles", Some(&editor_cookie)).await;
    assert_eq!(articles_status, StatusCode::OK);

    let (comments_status, _) = get_json(router, "/api/admin/comments", Some(&editor_cookie)).await;
    assert_eq!(comments_status, StatusCode::OK);
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
async fn admin_can_get_user_detail_with_related_articles() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (cookie, _) = admin_session(router.clone(), "admin").await;

    let (status, payload) = get_json(router, "/api/admin/users/2", Some(&cookie)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["user"]["id"], 2);
    assert_eq!(payload["user"]["username"], "editor");
    assert_eq!(payload["user"]["email"], "editor@example.com");
    assert_eq!(payload["user"]["role"], "editor");
    assert_eq!(payload["user"]["article_count"], 1);
    assert_eq!(payload["user"]["permissions"][0], "publish");
    assert_eq!(payload["recent_articles"][0]["id"], 1);
    assert_eq!(payload["recent_articles"][0]["title"], "Editor story");
    assert_eq!(payload["recent_articles"][0]["slug"], "editor-story");
    assert_eq!(payload["recent_articles"][0]["status"], "published");
    assert_eq!(
        payload["recent_articles"][0]["category"]["name"],
        "Technology"
    );
    assert_eq!(payload["roles"][0]["key"], "admin");
    assert_eq!(payload["permissions"][0]["key"], "publish");
}

#[tokio::test]
async fn admin_can_update_user_profile_and_role() {
    let redis = support::FakeRedis::start();
    let pool = seeded_pool().await;
    let router = support::router_with_redis(pool.clone(), &redis);
    let (cookie, csrf) = admin_session(router.clone(), "admin").await;

    let (status, payload) = json_request(
        router,
        Method::PUT,
        "/api/admin/users/2",
        &cookie,
        &csrf,
        r#"{"username":"editor-chief","email":"chief@example.com","role":"admin"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["user"]["username"], "editor-chief");
    assert_eq!(payload["user"]["email"], "chief@example.com");
    assert_eq!(payload["user"]["role"], "admin");
    assert!(payload["user"]["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "users"));

    let row: (String, String, String) = sqlx::query_as(db::sql(
        "SELECT username, email, role FROM users WHERE id = ?",
    ))
    .bind(2_i64)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, "editor-chief");
    assert_eq!(row.1, "chief@example.com");
    assert_eq!(row.2, "admin");
}

#[tokio::test]
async fn admin_update_user_rejects_duplicate_identity() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (cookie, csrf) = admin_session(router.clone(), "admin").await;

    let (status, payload) = json_request(
        router,
        Method::PUT,
        "/api/admin/users/2",
        &cookie,
        &csrf,
        r#"{"username":"admin","email":"admin@example.com","role":"editor"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(payload["code"], "conflict");
}

#[tokio::test]
async fn admin_rejects_invalid_login_names_without_spaces_or_non_ascii() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (cookie, csrf) = admin_session(router.clone(), "admin").await;

    let (create_status, create_payload) = json_request(
        router.clone(),
        Method::POST,
        "/api/admin/users",
        &cookie,
        &csrf,
        r#"{"username":"bad user","email":"bad@example.com","password":"admin-password","role":"writer"}"#,
    )
    .await;
    assert_eq!(create_status, StatusCode::BAD_REQUEST);
    assert_eq!(create_payload["code"], "invalid_params");
    assert_eq!(
        create_payload["message"],
        "登录名需为 3-64 位字母、数字、点、下划线或短横线，不能包含空格或中文"
    );

    let (update_status, update_payload) = json_request(
        router,
        Method::PUT,
        "/api/admin/users/2",
        &cookie,
        &csrf,
        r#"{"username":"中文名","email":"editor@example.com","role":"editor"}"#,
    )
    .await;
    assert_eq!(update_status, StatusCode::BAD_REQUEST);
    assert_eq!(update_payload["code"], "invalid_params");
    assert_eq!(
        update_payload["message"],
        "登录名需为 3-64 位字母、数字、点、下划线或短横线，不能包含空格或中文"
    );
}

#[tokio::test]
async fn admin_cannot_remove_own_admin_role_from_detail_update() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let (cookie, csrf) = admin_session(router.clone(), "admin").await;

    let (status, payload) = json_request(
        router,
        Method::PUT,
        "/api/admin/users/1",
        &cookie,
        &csrf,
        r#"{"username":"admin","email":"admin@example.com","role":"editor"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["code"], "invalid_params");
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
    let exists: i64 = sqlx::query_scalar(db::sql("SELECT COUNT(*) FROM users WHERE id = ?"))
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
