use std::{fs, process::Command};

fn write_config(path: &std::path::Path, db_path: &std::path::Path) {
    let db_path = db_path.display().to_string().replace('\\', "/");
    fs::write(
        path,
        format!(
            r#"
database:
  path: "{}"
session:
  secret: "custom-session-secret-with-32-bytes"
admin:
  init_password: "custom-admin-password"
"#,
            db_path
        ),
    )
    .unwrap();
}

fn blogweb() -> &'static str {
    env!("CARGO_BIN_EXE_blogweb")
}

#[test]
fn db_check_fails_without_writing_schema_migrations() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    let db = dir.path().join("blog.db");
    write_config(&config, &db);

    let output = Command::new(blogweb())
        .args(["db", "check", "-config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("schema_migrations"), "stderr: {stderr}");
    assert!(!db.exists(), "db check must not create target database");
}

#[test]
fn db_migrate_apply_creates_schema_and_db_check_passes() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    let db = dir.path().join("blog.db");
    write_config(&config, &db);

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
    assert!(db.exists());

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

#[test]
fn db_migrate_dry_run_does_not_create_target_database() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    let db = dir.path().join("blog.db");
    write_config(&config, &db);

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
    assert!(!db.exists(), "dry-run must not create target database");
}

#[test]
fn serve_web_fails_when_database_is_not_migrated_without_creating_db() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    let db = dir.path().join("blog.db");
    write_config(&config, &db);

    let output = Command::new(blogweb())
        .args(["serve-web", "-config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("schema_migrations"), "stderr: {stderr}");
    assert!(
        !db.exists(),
        "serve-web must not create or migrate the target database"
    );
}
