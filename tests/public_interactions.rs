use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use blogweb::{app, config::Config, db};
use serde_json::Value;
use tower::ServiceExt;

mod support;

async fn seeded_pool() -> db::DbPool {
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
            1, 'Public article', 'public-article', '# body', '', 'body', 1, 1,
            'published', 0, '2026-05-29T08:00:00Z',
            '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'
         ), (
            2, 'Second public article', 'second-public-article', '# body', '', 'body', 1, 1,
            'published', 0, '2026-05-28T08:00:00Z',
            '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'
         )",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn json_request(
    router: axum::Router,
    method: Method,
    uri: &str,
    body: &str,
) -> (StatusCode, Value) {
    json_request_with_cookie(router, method, uri, "anonymous_id=reader-1", body).await
}

async fn json_request_with_cookie(
    router: axum::Router,
    method: Method,
    uri: &str,
    cookie: &str,
    body: &str,
) -> (StatusCode, Value) {
    let response = router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("cookie", cookie)
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
async fn public_like_and_batch_status_use_anonymous_cookie() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);

    let (like_status, like_payload) = json_request(
        router.clone(),
        Method::POST,
        "/api/articles/public-article/like",
        r#"{"action":"like"}"#,
    )
    .await;
    assert_eq!(like_status, StatusCode::OK);
    assert_eq!(like_payload["liked"], true);
    assert_eq!(like_payload["like_count"], 1);

    let (batch_status, batch_payload) = json_request(
        router,
        Method::POST,
        "/api/likes/batch",
        r#"{"article_slugs":["public-article"]}"#,
    )
    .await;
    assert_eq!(batch_status, StatusCode::OK);
    assert_eq!(batch_payload["liked_map"]["public-article"], true);
}

#[tokio::test]
async fn public_bookmark_follow_newsletter_and_comment_work_with_cookie() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);

    let (bookmark_status, bookmark_payload) = json_request(
        router.clone(),
        Method::POST,
        "/api/articles/public-article/bookmark",
        r#"{"action":"bookmark"}"#,
    )
    .await;
    assert_eq!(bookmark_status, StatusCode::OK);
    assert_eq!(bookmark_payload["bookmarked"], true);
    assert_eq!(bookmark_payload["bookmark_count"], 1);

    let (follow_status, follow_payload) = json_request(
        router.clone(),
        Method::POST,
        "/api/authors/1/follow",
        r#"{"action":"follow"}"#,
    )
    .await;
    assert_eq!(follow_status, StatusCode::OK);
    assert_eq!(follow_payload["following"], true);
    assert_eq!(follow_payload["follower_count"], 1);

    let (subscribe_status, subscribe_payload) = json_request(
        router.clone(),
        Method::POST,
        "/api/newsletter/subscribe",
        r#"{"email":"Reader@Example.com"}"#,
    )
    .await;
    assert_eq!(subscribe_status, StatusCode::CREATED);
    assert_eq!(subscribe_payload["subscribed"], true);
    assert_eq!(subscribe_payload["email"], "reader@example.com");

    let (comment_status, comment_payload) = json_request(
        router,
        Method::POST,
        "/api/articles/public-article/comments",
        r#"{"author_name":"读者","content":"这是一条正常评论。"}"#,
    )
    .await;
    assert_eq!(comment_status, StatusCode::CREATED);
    assert_eq!(comment_payload["status"], "approved");
    assert_eq!(comment_payload["message"], "评论已发布");
}

#[tokio::test]
async fn reader_interactions_are_rate_limited_by_ip() {
    let redis = support::FakeRedis::start();
    let mut config = Config::default();
    config.redis.addr = redis.addr().to_string();
    config.rate_limit.like_ip_max_requests = 1;
    config.rate_limit.like_article_max_actions = 10;
    config.rate_limit.comment_ip_max_requests = 1;
    config.rate_limit.comment_article_max_actions = 10;
    let router = app::router_with_pool_and_config(
        seeded_pool().await,
        std::path::PathBuf::from("public/assets"),
        std::path::PathBuf::from("public/uploads"),
        config,
    );

    let (like_status, _) = json_request_with_cookie(
        router.clone(),
        Method::POST,
        "/api/articles/public-article/like",
        "anonymous_id=reader-1",
        r#"{"action":"like"}"#,
    )
    .await;
    assert_eq!(like_status, StatusCode::OK);

    let (second_like_status, second_like_payload) = json_request_with_cookie(
        router.clone(),
        Method::POST,
        "/api/articles/public-article/like",
        "anonymous_id=reader-2",
        r#"{"action":"like"}"#,
    )
    .await;
    assert_eq!(second_like_status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(second_like_payload["code"], "rate_limited");

    let (comment_status, _) = json_request_with_cookie(
        router.clone(),
        Method::POST,
        "/api/articles/second-public-article/comments",
        "anonymous_id=reader-1",
        r#"{"author_name":"读者","content":"这是一条正常评论。"}"#,
    )
    .await;
    assert_eq!(comment_status, StatusCode::CREATED);

    let (second_comment_status, second_comment_payload) = json_request_with_cookie(
        router,
        Method::POST,
        "/api/articles/second-public-article/comments",
        "anonymous_id=reader-2",
        r#"{"author_name":"读者","content":"另一条正常评论。"}"#,
    )
    .await;
    assert_eq!(second_comment_status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(second_comment_payload["code"], "rate_limited");
}

#[tokio::test]
async fn public_comment_rejects_sensitive_policy_terms() {
    let redis = support::FakeRedis::start();
    let router = support::router_with_redis(seeded_pool().await, &redis);

    let (status, payload) = json_request(
        router,
        Method::POST,
        "/api/articles/public-article/comments",
        r#"{"author_name":"读者","content":"b-l-o-o-d 内容不应通过"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["code"], "comment_policy_violation");
    assert_eq!(
        payload["message"],
        "评论包含血腥相关敏感内容，请修改后再提交"
    );
}
