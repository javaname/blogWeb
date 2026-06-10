use blogweb::db::{self, MigrationCheckStatus};

async fn fresh_pool() -> db::DbPool {
    db::connect_memory().await.unwrap()
}

#[tokio::test]
async fn migration_check_does_not_create_schema_migrations() {
    let pool = fresh_pool().await;

    let err = db::check_migrations(&pool).await.unwrap_err();
    assert!(
        err.to_string().contains("schema_migrations"),
        "unexpected error: {err}"
    );

    let table_count: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM information_schema.tables
         WHERE table_schema = current_schema()
           AND table_name = 'schema_migrations'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(table_count, 0);
}

#[tokio::test]
async fn migration_check_rejects_unregistered_schema_migrations() {
    let pool = fresh_pool().await;
    sqlx::query(
        "CREATE TABLE schema_migrations (
            version TEXT NOT NULL PRIMARY KEY,
            filename TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let err = db::check_migrations(&pool).await.unwrap_err();

    assert!(
        err.to_string().contains("not registered"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn apply_migrations_creates_schema_and_records_hashes() {
    let pool = fresh_pool().await;

    db::apply_migrations(&pool).await.unwrap();

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name
         FROM information_schema.tables
         WHERE table_schema = current_schema()
         ORDER BY table_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(tables.contains(&"users".to_string()));
    assert!(tables.contains(&"articles".to_string()));
    assert!(tables.contains(&"comments".to_string()));
    assert!(tables.contains(&"email_verification_codes".to_string()));
    assert!(tables.contains(&"schema_migrations".to_string()));

    let versions: Vec<String> =
        sqlx::query_scalar("SELECT version FROM schema_migrations ORDER BY version")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(versions, vec!["001", "002", "003", "004", "005", "006"]);
    assert_eq!(
        db::check_migrations(&pool).await.unwrap(),
        MigrationCheckStatus::Ready
    );
}

#[tokio::test]
async fn apply_migrations_is_idempotent_when_hashes_match() {
    let pool = fresh_pool().await;

    db::apply_migrations(&pool).await.unwrap();
    db::apply_migrations(&pool).await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM schema_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 6);
}

#[tokio::test]
async fn migration_check_rejects_recorded_hash_mismatch() {
    let pool = fresh_pool().await;

    db::apply_migrations(&pool).await.unwrap();
    sqlx::query("UPDATE schema_migrations SET sha256 = 'bad-hash' WHERE version = '001'")
        .execute(&pool)
        .await
        .unwrap();

    let err = db::check_migrations(&pool).await.unwrap_err();

    assert!(
        err.to_string().contains("hash mismatch"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn migration_check_rejects_registered_schema_with_missing_table() {
    let pool = fresh_pool().await;

    db::apply_migrations(&pool).await.unwrap();
    sqlx::query("DROP TABLE email_verification_codes")
        .execute(&pool)
        .await
        .unwrap();

    let err = db::check_migrations(&pool).await.unwrap_err();

    assert!(
        err.to_string()
            .contains("missing table email_verification_codes"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn migration_check_rejects_registered_schema_with_missing_column() {
    let pool = fresh_pool().await;

    db::apply_migrations(&pool).await.unwrap();
    sqlx::query("ALTER TABLE users DROP COLUMN email_verified_at")
        .execute(&pool)
        .await
        .unwrap();

    let err = db::check_migrations(&pool).await.unwrap_err();

    assert!(
        err.to_string()
            .contains("missing column users.email_verified_at"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn apply_migrations_registers_existing_schema_without_replaying_alter_columns() {
    let pool = fresh_pool().await;

    db::apply_migrations(&pool).await.unwrap();
    sqlx::query("DROP TABLE schema_migrations")
        .execute(&pool)
        .await
        .unwrap();

    db::apply_migrations(&pool).await.unwrap();

    let versions: Vec<String> =
        sqlx::query_scalar("SELECT version FROM schema_migrations ORDER BY version")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(versions, vec!["001", "002", "003", "004", "005", "006"]);
    assert_eq!(
        db::check_migrations(&pool).await.unwrap(),
        MigrationCheckStatus::Ready
    );
}

#[tokio::test]
async fn apply_migrations_rolls_back_registration_when_final_schema_check_fails() {
    let pool = fresh_pool().await;
    sqlx::query("CREATE TABLE users (id BIGSERIAL PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();

    let err = db::apply_migrations(&pool).await.unwrap_err();

    assert!(
        err.to_string().contains("missing column users.username"),
        "unexpected error: {err}"
    );
    let registered: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM information_schema.tables
         WHERE table_schema = current_schema()
           AND table_name = 'schema_migrations'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(registered, 0);
}
