use blogweb::{db, sqlite_sync};
use sqlx::{sqlite::SqlitePoolOptions, Row};

#[tokio::test]
async fn sync_sqlite_preserves_ids_and_resets_postgres_sequences() {
    let sqlite = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            email TEXT NULL,
            email_verified_at DATETIME NULL
        )",
    )
    .execute(&sqlite)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            slug TEXT NOT NULL UNIQUE,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&sqlite)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            slug TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL,
            cover_image TEXT NOT NULL DEFAULT '',
            excerpt TEXT NOT NULL DEFAULT '',
            category_id INTEGER NULL,
            author_id INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'draft',
            is_pinned INTEGER NOT NULL DEFAULT 0,
            published_at DATETIME NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&sqlite)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE likes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            anonymous_id TEXT NOT NULL,
            ip_address TEXT NOT NULL,
            user_agent TEXT NOT NULL DEFAULT '',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&sqlite)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE slug_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NULL,
            old_slug TEXT NOT NULL UNIQUE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&sqlite)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO users (id, username, password, role, created_at, email, email_verified_at)
         VALUES (7, 'admin', 'hash', 'admin', '2026-06-01 00:00:00', 'admin@example.com', NULL)",
    )
    .execute(&sqlite)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO categories (id, name, slug, sort_order, created_at)
         VALUES (5, 'Rust', 'rust', 10, '2026-06-01 00:00:00')",
    )
    .execute(&sqlite)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO articles (
            id, title, slug, content, cover_image, excerpt, category_id, author_id,
            status, is_pinned, published_at, created_at, updated_at
         ) VALUES (
            11, 'Hello PG', 'hello-pg', 'Body', '', 'Body', 5, 7,
            'published', 1, '2026-06-01 00:00:00', '2026-06-01 00:00:00', '2026-06-01 00:00:00'
         )",
    )
    .execute(&sqlite)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO likes (id, article_id, anonymous_id, ip_address, user_agent, created_at)
         VALUES (13, 11, 'reader-1', '', '', '2026-06-01 00:00:00')",
    )
    .execute(&sqlite)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO slug_history (id, article_id, old_slug, created_at)
         VALUES (17, 11, 'old-hello-pg', '2026-06-01 00:00:00')",
    )
    .execute(&sqlite)
    .await
    .unwrap();

    let target = db::connect_memory().await.unwrap();
    db::apply_migrations(&target).await.unwrap();

    let report = sqlite_sync::sync_pool(&sqlite, &target).await.unwrap();

    assert_eq!(report.imported_rows("users"), 1);
    assert_eq!(report.imported_rows("articles"), 1);

    let article = sqlx::query("SELECT id, title, is_pinned FROM articles WHERE slug = 'hello-pg'")
        .fetch_one(&target)
        .await
        .unwrap();
    assert_eq!(article.get::<i64, _>("id"), 11);
    assert_eq!(article.get::<String, _>("title"), "Hello PG");
    assert_eq!(article.get::<i64, _>("is_pinned"), 1);

    let next_article_id: i64 =
        sqlx::query_scalar("INSERT INTO articles (title, slug, content, author_id) VALUES ('Next', 'next', '', 7) RETURNING id")
            .fetch_one(&target)
            .await
            .unwrap();
    assert!(next_article_id > 11);
}
