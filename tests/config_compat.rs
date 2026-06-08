use std::fs;

use blogweb::config;

#[test]
fn load_config_keeps_go_defaults_and_seed_defaults_safe() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    fs::write(
        &path,
        r#"
server:
  port: 3000
database:
  url: "postgres://localhost:5432/blogweb"
redis:
  addr: "127.0.0.1:6379"
  pool_size: 10
session:
  secret: "replace-with-random-32-byte-secret"
  max_age: 86400
  idle_timeout: 7200
upload:
  dir: "public/uploads"
  max_size: 5242880
  allowed_types:
    - image/jpeg
    - image/png
    - image/gif
    - image/webp
admin:
  init_password: "custom-admin-password"
"#,
    )
    .unwrap();

    let cfg = config::load(&path).unwrap();

    assert_eq!(cfg.server.port, 3000);
    assert_eq!(cfg.database.url, "postgres://localhost:5432/blogweb");
    assert_eq!(cfg.redis.addr, "127.0.0.1:6379");
    assert_eq!(cfg.redis.pool_size, 10);
    assert_eq!(cfg.session.max_age, 86400);
    assert_eq!(cfg.upload.dir, "public/uploads");
    assert_eq!(
        cfg.upload.allowed_types,
        vec!["image/jpeg", "image/png", "image/gif", "image/webp"]
    );
    assert!(!cfg.seed.demo_content_enabled);
    assert!(!cfg.seed.allow_insecure_admin_password);
}

#[test]
fn load_config_merges_with_go_defaults_when_sections_are_omitted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    fs::write(
        &path,
        r#"
database:
  url: "postgres://postgres:secret@localhost:5432/custom_blog"
session:
  secret: "custom-session-secret-with-32-bytes"
admin:
  init_password: "custom-admin-password"
"#,
    )
    .unwrap();

    let cfg = config::load(&path).unwrap();

    assert_eq!(cfg.server.port, 3000);
    assert_eq!(
        cfg.database.url,
        "postgres://postgres:secret@localhost:5432/custom_blog"
    );
    assert_eq!(cfg.redis.addr, "127.0.0.1:6379");
    assert_eq!(cfg.session.secret, "custom-session-secret-with-32-bytes");
    assert_eq!(cfg.session.max_age, 86400);
    assert_eq!(cfg.upload.dir, "public/uploads");
    assert_eq!(cfg.admin.init_username, "admin");
    assert_eq!(cfg.admin.init_password, "custom-admin-password");
}

#[test]
fn config_rejects_insecure_default_admin_password_when_seed_does_not_allow_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    fs::write(
        &path,
        r#"
session:
  secret: "custom-session-secret-with-32-bytes"
"#,
    )
    .unwrap();

    let err = config::load(&path).unwrap_err();

    assert!(
        err.to_string().contains("admin.init_password"),
        "unexpected error: {err}"
    );
}
