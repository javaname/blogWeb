use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
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
            1, 'Public article', 'public-article', '# body', '', 'body', 1, 1,
            'published', 0, '2026-05-29T08:00:00Z',
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
    let response = router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("cookie", "anonymous_id=reader-1")
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
    let router = app::router_with_pool(seeded_pool().await);

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
    let router = app::router_with_pool(seeded_pool().await);

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
