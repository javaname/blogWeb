use axum::body::Body;
use axum::http::{header::SET_COOKIE, Request, StatusCode};
use blogweb::db;
use serde_json::Value;
use sqlx::Pool;
use tower::ServiceExt;

mod support;

async fn seeded_pool() -> Pool<sqlx::Sqlite> {
    let pool = db::connect_memory().await.unwrap();
    db::apply_migrations(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password, role, email, created_at)
         VALUES (1, 'admin', ?, 'admin', '', '2026-05-29T00:00:00Z')",
    )
    .bind(support::ADMIN_PASSWORD_HASH)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO categories (id, name, slug, sort_order, created_at)
         VALUES
         (1, 'Technology', 'technology', 0, '2026-05-29T00:00:00Z'),
         (2, 'Design', 'design', 1, '2026-05-29T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO articles (
            id, title, slug, content, cover_image, excerpt, category_id, author_id,
            status, is_pinned, published_at, created_at, updated_at
         ) VALUES
         (1, 'Published dashboard article', 'published-dashboard-article', '# body', '', 'body', 1, 1,
          'published', 1, '2026-05-29T08:00:00Z', '2026-05-29T00:00:00Z', '2026-05-29T01:00:00Z'),
         (2, 'Draft dashboard article', 'draft-dashboard-article', '# draft', '', 'draft', 2, 1,
          'draft', 0, NULL, '2026-05-29T00:00:00Z', '2026-05-29T02:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO likes (article_id, anonymous_id, ip_address, user_agent, created_at)
         VALUES (1, 'reader-1', '127.0.0.1', '', '2026-05-29T03:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO comments (
            id, article_id, author_name, content, status, anonymous_id, ip_address,
            user_agent, created_at, updated_at
         ) VALUES (
            1, 1, '读者', '统计接口需要看到这条评论。', 'approved', 'reader-1',
            '127.0.0.1', '', '2026-05-29T04:00:00Z', '2026-05-29T04:00:00Z'
         )",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn admin_cookie(router: axum::Router) -> String {
    let response = router
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

    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .find(|cookie| cookie.starts_with("admin_session="))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap()
        .to_string()
}

async fn get_json(router: axum::Router, uri: &str, cookie: &str) -> (StatusCode, Value) {
    let response = router
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
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
async fn admin_read_routes_require_login() {
    let redis = support::FakeRedis::start();
    let response = support::router_with_redis(seeded_pool().await, &redis)
        .oneshot(
            Request::builder()
                .uri("/api/admin/dashboard")
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
async fn admin_dashboard_returns_real_metrics() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let cookie = admin_cookie(router.clone()).await;

    let (status, payload) = get_json(router, "/api/admin/dashboard", &cookie).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["stats"]["total_articles"], 2);
    assert_eq!(payload["stats"]["published_articles"], 1);
    assert_eq!(payload["stats"]["draft_articles"], 1);
    assert_eq!(payload["stats"]["total_comments"], 1);
    assert_eq!(payload["stats"]["total_likes"], 1);
    assert!(payload["activity"].as_array().unwrap().len() >= 1);
    assert_eq!(payload["views_trend"].as_array().unwrap().len(), 30);
}

#[tokio::test]
async fn admin_settings_returns_public_runtime_policy_without_secrets() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let cookie = admin_cookie(router.clone()).await;

    let (status, payload) = get_json(router, "/api/admin/settings", &cookie).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["site"]["title"], "个人博客");
    assert_eq!(payload["publishing"]["default_author"], "admin");
    assert_eq!(payload["upload"]["allow_svg"], false);
    let text = payload.to_string();
    assert!(!text.contains("change-this-session-secret-to-32-bytes"));
    assert!(!text.contains("change-me-123456"));
}

#[tokio::test]
async fn admin_articles_categories_and_comments_lists_match_go_shape() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);
    let cookie = admin_cookie(router.clone()).await;

    let (articles_status, articles) = get_json(
        router.clone(),
        "/api/admin/articles?page=1&page_size=10",
        &cookie,
    )
    .await;
    assert_eq!(articles_status, StatusCode::OK);
    assert_eq!(articles["total"], 2);
    assert_eq!(articles["page"], 1);
    assert_eq!(articles["page_size"], 10);
    assert_eq!(articles["list"][0]["title"], "Draft dashboard article");
    assert_eq!(articles["list"][0]["author"]["username"], "admin");
    assert_eq!(articles["list"][1]["category"]["name"], "Technology");
    assert_eq!(articles["list"][1]["like_count"], 1);

    let (categories_status, categories) =
        get_json(router.clone(), "/api/admin/categories", &cookie).await;
    assert_eq!(categories_status, StatusCode::OK);
    assert_eq!(categories.as_array().unwrap().len(), 2);
    assert_eq!(categories[0]["name"], "Technology");
    assert_eq!(categories[0]["article_count"], 1);

    let (comments_status, comments) =
        get_json(router, "/api/admin/comments?page=1&page_size=20", &cookie).await;
    assert_eq!(comments_status, StatusCode::OK);
    assert_eq!(comments["total"], 1);
    assert_eq!(
        comments["list"][0]["article_title"],
        "Published dashboard article"
    );
    assert_eq!(comments["list"][0]["author_name"], "读者");
}
