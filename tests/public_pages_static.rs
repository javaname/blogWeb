use axum::body::Body;
use axum::http::{Request, StatusCode};
use blogweb::{app, db};
use sqlx::Pool;
use std::fs;
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
         (1, 'Rust Migration Baseline', 'rust-migration-baseline',
          '# Baseline\n\n<script>alert(1)</script>\n\nStable text.', '',
          'Baseline <script alert 1 </script Stable text.', 1, 1,
          'published', 0, '2026-05-29T08:00:00Z',
          '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'),
         (2, 'Design Systems', 'design-systems',
          '# Design', '', 'Design excerpt', 2, 1,
          'published', 0, '2026-05-28T08:00:00Z',
          '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn home_page_renders_public_articles_as_html() {
    let response = app::router_with_pool(seeded_pool().await)
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    let body = body_text(response).await;
    assert!(body.contains("Rust Migration Baseline"), "{body}");
    assert!(body.contains("/articles/rust-migration-baseline"), "{body}");
}

#[tokio::test]
async fn article_page_renders_sanitized_html_without_escaping() {
    let response = app::router_with_pool(seeded_pool().await)
        .oneshot(
            Request::builder()
                .uri("/articles/rust-migration-baseline")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("<h1>Baseline</h1>"), "{body}");
    assert!(!body.contains("<script>"), "{body}");
    assert!(!body.contains("&lt;h1&gt;Baseline"), "{body}");
}

#[tokio::test]
async fn category_page_only_renders_matching_category_articles() {
    let response = app::router_with_pool(seeded_pool().await)
        .oneshot(
            Request::builder()
                .uri("/categories/technology")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("Technology"), "{body}");
    assert!(body.contains("Rust Migration Baseline"), "{body}");
    assert!(!body.contains("Design Systems"), "{body}");
}

#[tokio::test]
async fn static_assets_and_uploads_are_served_and_reject_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let assets = dir.path().join("assets");
    let uploads = dir.path().join("uploads");
    fs::create_dir_all(&assets).unwrap();
    fs::create_dir_all(&uploads).unwrap();
    fs::write(assets.join("site.js"), "console.log('asset');").unwrap();
    fs::write(uploads.join("cover.txt"), "upload file").unwrap();

    let router = app::router_with_pool_and_paths(seeded_pool().await, assets, uploads);

    let asset = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/assets/site.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(asset.status(), StatusCode::OK);
    assert_eq!(body_text(asset).await, "console.log('asset');");

    let upload = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/uploads/cover.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upload.status(), StatusCode::OK);
    assert_eq!(body_text(upload).await, "upload file");

    let traversal = router
        .oneshot(
            Request::builder()
                .uri("/assets/../Cargo.toml")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(traversal.status(), StatusCode::NOT_FOUND);
}
