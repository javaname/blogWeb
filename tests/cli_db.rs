use blogweb::db;
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn write_config(path: &std::path::Path, database_url: &str) {
    fs::write(
        path,
        format!(
            r#"
database:
  url: "{}"
session:
  secret: "custom-session-secret-with-32-bytes"
admin:
  init_password: "custom-admin-password"
"#,
            database_url
        ),
    )
    .unwrap();
}

fn blogweb() -> &'static str {
    env!("CARGO_BIN_EXE_blogweb")
}

#[tokio::test]
async fn db_check_fails_without_writing_schema_migrations() {
    let Some(database_url) = temporary_database_url().await else {
        eprintln!("skipping CLI database test; BLOGWEB_TEST_DATABASE_URL is not set");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    write_config(&config, &database_url);

    let output = Command::new(blogweb())
        .args(["db", "check", "-config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("schema_migrations"), "stderr: {stderr}");
    assert_eq!(schema_migrations_table_count(&database_url).await, 0);
}

#[tokio::test]
async fn db_migrate_apply_creates_schema_and_db_check_passes() {
    let Some(database_url) = temporary_database_url().await else {
        eprintln!("skipping CLI database test; BLOGWEB_TEST_DATABASE_URL is not set");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    write_config(&config, &database_url);

    let apply = Command::new(blogweb())
        .args(["db", "migrate", "--apply", "-config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        apply.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&apply.stderr)
    );
    assert_eq!(schema_migrations_table_count(&database_url).await, 1);

    let check = Command::new(blogweb())
        .args(["db", "check", "-config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&check.stderr)
    );
}

#[tokio::test]
async fn db_migrate_dry_run_does_not_create_target_schema_migrations() {
    let Some(database_url) = temporary_database_url().await else {
        eprintln!("skipping CLI database test; BLOGWEB_TEST_DATABASE_URL is not set");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    write_config(&config, &database_url);

    let output = Command::new(blogweb())
        .args(["db", "migrate", "--dry-run", "-config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(schema_migrations_table_count(&database_url).await, 0);
}

#[tokio::test]
async fn serve_web_fails_when_database_is_not_migrated_without_creating_schema_migrations() {
    let Some(database_url) = temporary_database_url().await else {
        eprintln!("skipping CLI database test; BLOGWEB_TEST_DATABASE_URL is not set");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    write_config(&config, &database_url);

    let output = Command::new(blogweb())
        .args(["serve-web", "-config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("schema_migrations"), "stderr: {stderr}");
    assert_eq!(schema_migrations_table_count(&database_url).await, 0);
}

async fn temporary_database_url() -> Option<String> {
    let base_url = std::env::var("BLOGWEB_TEST_DATABASE_URL").ok()?;
    let schema = format!("cli_test_{}", unique_suffix());
    let pool = db::connect_existing(&base_url).await.ok()?;
    sqlx::query(&format!(r#"CREATE SCHEMA "{}""#, schema))
        .execute(&pool)
        .await
        .ok()?;
    Some(with_search_path(&base_url, &schema))
}

async fn schema_migrations_table_count(database_url: &str) -> i64 {
    let pool = db::connect_existing(database_url).await.unwrap();
    sqlx::query_scalar(
        "SELECT count(*)
         FROM information_schema.tables
         WHERE table_schema = current_schema()
           AND table_name = 'schema_migrations'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
}

fn with_search_path(base_url: &str, schema: &str) -> String {
    let separator = if base_url.contains('?') { '&' } else { '?' };
    format!("{base_url}{separator}options=-csearch_path%3D{schema}")
}

fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".into())
}
