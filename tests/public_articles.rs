use axum::body::Body;
use axum::http::{Request, StatusCode};
use blogweb::{app, db};
use serde_json::Value;
use sqlx::Pool;
use tower::ServiceExt;

async fn seeded_pool() -> Pool<sqlx::Sqlite> {
    let pool = db::connect_memory().await.unwrap();
    db::apply_migrations(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password, role, created_at)
         VALUES (1, 'admin', 'hash', 'admin', '2026-05-29T00:00:00Z')",
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
        "INSERT INTO categories (id, name, slug, sort_order, created_at)
         VALUES (2, 'Design', 'design', 1, '2026-05-29T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO articles (
            id, title, slug, content, cover_image, excerpt, category_id, author_id,
            status, is_pinned, published_at, created_at, updated_at
         ) VALUES (
            1, 'Rust Migration Baseline', 'rust-migration-baseline',
            '# Baseline\n\n<script>alert(1)</script>\n\nStable text.', '',
            'Baseline <script alert 1 </script Stable text.', 1, 1,
            'published', 0, '2026-05-29T08:00:00Z',
            '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'
         )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO slug_history (article_id, old_slug) VALUES (1, 'old-rust-baseline')")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

async fn seeded_pool_with_extra_articles() -> Pool<sqlx::Sqlite> {
    let pool = seeded_pool().await;
    sqlx::query(
        "INSERT INTO articles (
            id, title, slug, content, cover_image, excerpt, category_id, author_id,
            status, is_pinned, published_at, created_at, updated_at
         ) VALUES
         (2, 'Design Draft', 'design-draft', 'draft', '', 'draft', 2, 1,
          'draft', 0, NULL, '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'),
         (3, 'Future Rust', 'future-rust', 'future', '', 'future', 1, 1,
          'published', 0, '2999-01-01T00:00:00Z', '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'),
         (4, 'Design Systems', 'design-systems', 'design', '', 'design', 2, 1,
          'published', 0, '2026-05-28T08:00:00Z', '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

#[tokio::test]
async fn public_article_list_matches_go_golden_body() {
    let pool = seeded_pool().await;
    let response = app::router_with_pool(pool)
        .oneshot(
            Request::builder()
                .uri("/api/articles?limit=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let actual: Value = serde_json::from_slice(&body).unwrap();
    let golden: Value =
        serde_json::from_str(include_str!("../tests/golden/http/public_articles.json")).unwrap();

    assert_eq!(actual, golden["body"]);
}

#[tokio::test]
async fn public_article_list_filters_by_category_and_keyword_and_hides_unpublished() {
    let pool = seeded_pool_with_extra_articles().await;
    let response = app::router_with_pool(pool)
        .oneshot(
            Request::builder()
                .uri("/api/articles?category=technology&keyword=Migration&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let actual: Value = serde_json::from_slice(&body).unwrap();
    let list = actual["list"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["slug"], "rust-migration-baseline");
}

#[tokio::test]
async fn public_article_detail_redirects_historical_slug() {
    let pool = seeded_pool().await;
    let response = app::router_with_pool(pool)
        .oneshot(
            Request::builder()
                .uri("/api/articles/old-rust-baseline")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/api/articles/rust-migration-baseline"
    );
}

#[tokio::test]
async fn public_article_list_uses_go_cursor_contract() {
    let pool = seeded_pool_with_extra_articles().await;
    let first = app::router_with_pool(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/api/articles?limit=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let first_body = axum::body::to_bytes(first.into_body(), usize::MAX)
        .await
        .unwrap();
    let first_json: Value = serde_json::from_slice(&first_body).unwrap();
    assert_eq!(first_json["has_more"], true);
    assert_eq!(first_json["list"][0]["slug"], "rust-migration-baseline");
    let cursor = first_json["next_cursor"].as_str().unwrap();
    assert!(cursor.contains("published_at"));
    assert!(cursor.contains("is_pinned"));
    assert!(cursor.contains("id"));

    let second = app::router_with_pool(pool)
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/articles?limit=1&cursor={}",
                    url_escape(cursor)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let second_body = axum::body::to_bytes(second.into_body(), usize::MAX)
        .await
        .unwrap();
    let second_json: Value = serde_json::from_slice(&second_body).unwrap();
    assert_eq!(second_json["has_more"], false);
    assert_eq!(second_json["list"][0]["slug"], "design-systems");
}

fn url_escape(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('{', "%7B")
        .replace('}', "%7D")
        .replace('"', "%22")
        .replace(':', "%3A")
        .replace(',', "%2C")
        .replace(' ', "%20")
}

#[tokio::test]
async fn public_article_detail_matches_go_golden_body_shape() {
    let pool = seeded_pool().await;
    let response = app::router_with_pool(pool)
        .oneshot(
            Request::builder()
                .uri("/api/articles/rust-migration-baseline")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let mut actual: Value = serde_json::from_slice(&body).unwrap();
    actual["created_at"] = Value::String("<TIMESTAMP>".into());
    actual["updated_at"] = Value::String("<TIMESTAMP>".into());
    let golden: Value = serde_json::from_str(include_str!(
        "../tests/golden/http/public_article_detail.json"
    ))
    .unwrap();

    assert_eq!(actual, golden["body"]);
}
