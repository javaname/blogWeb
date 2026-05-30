use axum::body::Body;
use axum::http::{header::CONTENT_TYPE, Request, StatusCode};
use blogweb::{config::Config, db, mcp};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};
use tower::ServiceExt;

const SESSION_SECRET: &str = "custom-session-secret-with-32-bytes";

fn blogweb() -> &'static str {
    env!("CARGO_BIN_EXE_blogweb")
}

fn write_config(path: &std::path::Path, db_path: &std::path::Path) {
    let db_path = db_path.display().to_string().replace('\\', "/");
    fs::write(
        path,
        format!(
            r#"
database:
  path: "{}"
session:
  secret: "{}"
admin:
  init_password: "custom-admin-password"
"#,
            db_path, SESSION_SECRET
        ),
    )
    .unwrap();
}

#[tokio::test]
async fn mcp_issue_token_cli_stores_hmac_hash_and_revoke_disables_client() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    let db_path = dir.path().join("blog.db");
    write_config(&config, &db_path);

    let migrate = Command::new(blogweb())
        .args(["db", "migrate", "--apply", "-config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        migrate.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&migrate.stderr)
    );

    let issued = Command::new(blogweb())
        .args(["mcp", "issue-token", "-config"])
        .arg(&config)
        .args([
            "-name",
            "golden-reader",
            "-scopes",
            "blog.read,blog.category.read",
            "-transport",
            "http",
        ])
        .output()
        .unwrap();
    assert!(
        issued.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&issued.stderr)
    );
    let stdout = String::from_utf8_lossy(&issued.stdout);
    assert!(stdout.contains("name=golden-reader"), "stdout: {stdout}");
    assert!(stdout.contains("transport=http"), "stdout: {stdout}");
    let token = stdout
        .lines()
        .find_map(|line| line.strip_prefix("token="))
        .expect("token line should be printed");
    assert!(token.len() >= 24, "token too short: {token}");

    let pool = db::connect_existing(db_path.to_str().unwrap())
        .await
        .unwrap();
    let row = sqlx::query(
        "SELECT token_hash, scopes, transport, is_enabled
         FROM mcp_clients WHERE name = ?",
    )
    .bind("golden-reader")
    .fetch_one(&pool)
    .await
    .unwrap();
    let token_hash: String = row.get("token_hash");
    assert_ne!(token_hash, token, "plaintext token must not be stored");
    assert_eq!(token_hash, hmac_sha256_hex(SESSION_SECRET, token));
    assert_eq!(
        row.get::<String, _>("scopes"),
        r#"["blog.read","blog.category.read"]"#
    );
    assert_eq!(row.get::<String, _>("transport"), "http");
    assert_eq!(row.get::<i64, _>("is_enabled"), 1);

    let revoked = Command::new(blogweb())
        .args(["mcp", "revoke-token", "-config"])
        .arg(&config)
        .args(["-name", "golden-reader"])
        .output()
        .unwrap();
    assert!(
        revoked.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&revoked.stderr)
    );

    let enabled: i64 = sqlx::query_scalar("SELECT is_enabled FROM mcp_clients WHERE name = ?")
        .bind("golden-reader")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(enabled, 0);
}

#[tokio::test]
async fn mcp_http_missing_token_matches_go_golden() {
    let pool = migrated_pool().await;
    let response = mcp::router_with_pool_and_config(pool, Config::default())
        .oneshot(mcp_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            None,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let actual: Value = serde_json::from_slice(&body).unwrap();
    let golden: Value =
        serde_json::from_str(include_str!("../tests/golden/mcp/http_missing_token.json")).unwrap();
    assert_eq!(actual, golden["body"]);
}

#[tokio::test]
async fn mcp_http_initialize_with_bearer_token_matches_go_golden() {
    let pool = migrated_pool().await;
    let token = "reader-token-for-rust-mcp";
    sqlx::query(
        "INSERT INTO mcp_clients
         (name, token_hash, scopes, transport, is_enabled, created_at, updated_at)
         VALUES (?, ?, ?, 'http', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind("golden-reader")
    .bind(hmac_sha256_hex(&Config::default().session.secret, token))
    .bind(r#"["blog.read","blog.category.read"]"#)
    .execute(&pool)
    .await
    .unwrap();

    let response = mcp::router_with_pool_and_config(pool.clone(), Config::default())
        .oneshot(mcp_request(
            r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}"#,
            Some(token),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let actual: Value = serde_json::from_slice(&body).unwrap();
    let golden: Value =
        serde_json::from_str(include_str!("../tests/golden/mcp/http_initialize.json")).unwrap();
    assert_eq!(actual, golden["body"]);

    let last_used: Option<String> =
        sqlx::query_scalar("SELECT last_used_at FROM mcp_clients WHERE name = ?")
            .bind("golden-reader")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(last_used.is_some());
}

#[tokio::test]
async fn mcp_http_resources_read_returns_site_categories_articles_and_hides_future_posts() {
    let pool = seeded_content_pool().await;
    let token = "resource-reader-token";
    insert_mcp_client(
        &pool,
        "resource-reader",
        token,
        &["blog.read", "blog.category.read"],
    )
    .await;

    let site = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":10,"method":"resources/read","params":{"uri":"blog://site/meta"}}"#,
        token,
    )
    .await;
    assert_eq!(site.0, StatusCode::OK);
    assert_eq!(site.1["result"]["title"], "个人博客");
    assert_eq!(site.1["result"]["version"], "v6");

    let categories = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":11,"method":"resources/read","params":{"uri":"blog://categories"}}"#,
        token,
    )
    .await;
    assert_eq!(categories.0, StatusCode::OK);
    assert_eq!(categories.1["result"]["list"][0]["slug"], "technology");

    let article = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":12,"method":"resources/read","params":{"uri":"blog://articles/rust-migration-baseline"}}"#,
        token,
    )
    .await;
    assert_eq!(article.0, StatusCode::OK);
    assert_eq!(article.1["result"]["title"], "Rust Migration Baseline");
    assert_eq!(article.1["result"]["category"]["slug"], "technology");
    assert!(!article.1["result"]["content_html"]
        .as_str()
        .unwrap()
        .contains("<script"));

    let category_articles = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":13,"method":"resources/read","params":{"uri":"blog://categories/technology/articles"}}"#,
        token,
    )
    .await;
    assert_eq!(category_articles.0, StatusCode::OK);
    let list = category_articles.1["result"]["list"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["slug"], "rust-migration-baseline");

    let future = perform_mcp_json(
        pool,
        r#"{"jsonrpc":"2.0","id":14,"method":"resources/read","params":{"uri":"blog://articles/future-rust"}}"#,
        token,
    )
    .await;
    assert_eq!(future.0, StatusCode::NOT_FOUND);
    assert_eq!(future.1["error"]["data"]["code"], "not_found");
}

#[tokio::test]
async fn mcp_http_read_tools_list_get_categories_and_preview_markdown() {
    let pool = seeded_content_pool().await;
    let token = "tool-reader-token";
    insert_mcp_client(
        &pool,
        "tool-reader",
        token,
        &["blog.read", "blog.category.read", "blog.draft.write"],
    )
    .await;

    let tools = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":20,"method":"tools/list","params":{}}"#,
        token,
    )
    .await;
    assert_eq!(tools.0, StatusCode::OK);
    let tool_names = tools.1["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"list_articles"));
    assert!(tool_names.contains(&"preview_markdown"));
    assert!(tool_names.contains(&"create_article_draft"));

    let list_articles = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"list_articles","arguments":{"category":"technology","limit":10}}}"#,
        token,
    )
    .await;
    assert_eq!(list_articles.0, StatusCode::OK);
    assert_eq!(
        list_articles.1["result"]["list"][0]["slug"],
        "rust-migration-baseline"
    );

    let get_article = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"get_article","arguments":{"slug":"rust-migration-baseline"}}}"#,
        token,
    )
    .await;
    assert_eq!(get_article.0, StatusCode::OK);
    assert_eq!(get_article.1["result"]["title"], "Rust Migration Baseline");

    let list_categories = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"list_categories","arguments":{}}}"#,
        token,
    )
    .await;
    assert_eq!(list_categories.0, StatusCode::OK);
    assert_eq!(list_categories.1["result"]["list"][0]["name"], "Technology");

    let preview = perform_mcp_json(
        pool,
        r##"{"jsonrpc":"2.0","id":24,"method":"tools/call","params":{"name":"preview_markdown","arguments":{"content":"# Hi\n<script>alert(1)</script>\n[bad](javascript:alert(1))"}}}"##,
        token,
    )
    .await;
    assert_eq!(preview.0, StatusCode::OK);
    let html = preview.1["result"]["content_html"].as_str().unwrap();
    assert!(html.contains("<h1>Hi</h1>"));
    assert!(!html.to_ascii_lowercase().contains("<script"));
    assert!(!html.to_ascii_lowercase().contains("javascript:"));
}

#[tokio::test]
async fn mcp_http_rejects_invalid_origin_accept_and_forbidden_scope_like_go() {
    let pool = seeded_content_pool().await;
    let token = "limited-reader-token";
    insert_mcp_client(&pool, "limited-reader", token, &["blog.read"]).await;

    let invalid_origin = mcp::router_with_pool_and_config(pool.clone(), Config::default())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .header("MCP-Protocol-Version", "2025-11-25")
                .header("Origin", "https://evil.example")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":30,"method":"tools/list","params":{}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_origin.status(), StatusCode::FORBIDDEN);

    let invalid_accept = mcp::router_with_pool_and_config(pool.clone(), Config::default())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("Accept", "text/plain")
                .header("MCP-Protocol-Version", "2025-11-25")
                .header("Origin", "https://chatgpt.com")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":31,"method":"tools/list","params":{}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_accept.status(), StatusCode::BAD_REQUEST);

    let forbidden_scope = mcp::router_with_pool_and_config(pool, Config::default())
        .oneshot(mcp_request(
            r#"{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"name":"publish_article","arguments":{"id":1}}}"#,
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(forbidden_scope.status(), StatusCode::FORBIDDEN);
    assert!(forbidden_scope
        .headers()
        .get("www-authenticate")
        .unwrap()
        .to_str()
        .unwrap()
        .contains(r#"insufficient_scope", scope="blog.publish"#));
}

#[tokio::test]
async fn mcp_http_write_tools_create_update_publish_article_with_admin_author() {
    let pool = seeded_writer_pool().await;
    let token = "writer-token";
    insert_mcp_client(
        &pool,
        "writer",
        token,
        &["blog.draft.write", "blog.publish", "blog.read"],
    )
    .await;

    let created = perform_mcp_json(
        pool.clone(),
        r##"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"create_article_draft","arguments":{"title":"Draft Title","content":"# body","category_id":1}}}"##,
        token,
    )
    .await;
    assert_eq!(created.0, StatusCode::OK);
    let article_id = created.1["result"]["id"].as_i64().unwrap();
    assert_eq!(created.1["result"]["slug"], "draft-title");

    let author_id: i64 = sqlx::query_scalar("SELECT author_id FROM articles WHERE id = ?")
        .bind(article_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(author_id, 2, "MCP draft should use configured admin");

    let updated = perform_mcp_json(
        pool.clone(),
        format!(
            r###"{{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{{"name":"update_article","arguments":{{"id":{article_id},"title":"Published Title","content":"## updated"}}}}}}"###
        ),
        token,
    )
    .await;
    assert_eq!(updated.0, StatusCode::OK);
    assert_eq!(updated.1["result"]["slug"], "published-title");

    let published = perform_mcp_json(
        pool.clone(),
        format!(
            r#"{{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{{"name":"publish_article","arguments":{{"id":{article_id}}}}}}}"#
        ),
        token,
    )
    .await;
    assert_eq!(published.0, StatusCode::OK);
    assert_eq!(published.1["result"]["status"], "published");

    let public = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"get_article","arguments":{"slug":"published-title"}}}"#,
        token,
    )
    .await;
    assert_eq!(public.0, StatusCode::OK);
    assert_eq!(public.1["result"]["title"], "Published Title");

    let old_slug: Option<i64> =
        sqlx::query_scalar("SELECT article_id FROM slug_history WHERE old_slug = ?")
            .bind("draft-title")
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(old_slug, Some(article_id));
}

#[tokio::test]
async fn mcp_http_write_tools_reject_unsafe_cover_and_create_update_categories() {
    let pool = seeded_writer_pool().await;
    let token = "category-writer-token";
    insert_mcp_client(
        &pool,
        "category-writer",
        token,
        &[
            "blog.draft.write",
            "blog.category.write",
            "blog.category.read",
        ],
    )
    .await;

    let unsafe_cover = perform_mcp_json(
        pool.clone(),
        r##"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"create_article_draft","arguments":{"title":"Unsafe Cover","content":"# body","cover_image":"http://evil.example/x.png"}}}"##,
        token,
    )
    .await;
    assert_eq!(unsafe_cover.0, StatusCode::BAD_REQUEST);
    assert_eq!(unsafe_cover.1["error"]["data"]["code"], "invalid_params");

    let traversal_cover = perform_mcp_json(
        pool.clone(),
        r##"{"jsonrpc":"2.0","id":51,"method":"tools/call","params":{"name":"create_article_draft","arguments":{"title":"Traversal Cover","content":"# body","cover_image":"/uploads/../../evil.png"}}}"##,
        token,
    )
    .await;
    assert_eq!(traversal_cover.0, StatusCode::BAD_REQUEST);

    let created = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":52,"method":"tools/call","params":{"name":"create_category","arguments":{"name":"Rust Notes","slug":"rust-notes"}}}"#,
        token,
    )
    .await;
    assert_eq!(created.0, StatusCode::OK);
    let category_id = created.1["result"]["id"].as_i64().unwrap();
    assert_eq!(created.1["result"]["slug"], "rust-notes");

    let updated = perform_mcp_json(
        pool.clone(),
        format!(
            r#"{{"jsonrpc":"2.0","id":53,"method":"tools/call","params":{{"name":"update_category","arguments":{{"id":{category_id},"name":"Rust Deep Dives","slug":"rust-deep-dives","sort_order":9}}}}}}"#
        ),
        token,
    )
    .await;
    assert_eq!(updated.0, StatusCode::OK);
    assert_eq!(updated.1["result"]["name"], "Rust Deep Dives");
    assert_eq!(updated.1["result"]["sort_order"], 9);

    let categories = perform_mcp_json(
        pool,
        r#"{"jsonrpc":"2.0","id":54,"method":"tools/call","params":{"name":"list_categories","arguments":{}}}"#,
        token,
    )
    .await;
    assert_eq!(categories.0, StatusCode::OK);
    assert!(categories.1["result"]["list"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["slug"] == "rust-deep-dives"));
}

#[tokio::test]
async fn mcp_http_upload_image_stores_valid_png_and_rejects_invalid_payloads() {
    let pool = migrated_pool().await;
    let token = "upload-token";
    insert_mcp_client(&pool, "uploader", token, &["blog.upload"]).await;
    let upload_dir = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.upload.dir = upload_dir.path().display().to_string();

    let valid = perform_mcp_json_with_config(
        pool.clone(),
        config.clone(),
        r#"{"jsonrpc":"2.0","id":60,"method":"tools/call","params":{"name":"upload_image","arguments":{"filename":"ok.png","mime_type":"image/png","content_base64":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/lAbQwQAAAABJRU5ErkJggg=="}}}"#,
        token,
    )
    .await;
    assert_eq!(valid.0, StatusCode::OK);
    let url = valid.1["result"]["url"].as_str().unwrap();
    assert!(url.starts_with("/uploads/"));
    assert_eq!(valid.1["result"]["mime_type"], "image/png");
    assert!(valid.1["result"]["size"].as_i64().unwrap() > 0);
    let stored = upload_dir.path().join(url.trim_start_matches("/uploads/"));
    assert!(stored.exists(), "uploaded file should exist at {stored:?}");

    let fake = perform_mcp_json_with_config(
        pool.clone(),
        config.clone(),
        r#"{"jsonrpc":"2.0","id":61,"method":"tools/call","params":{"name":"upload_image","arguments":{"filename":"fake.png","mime_type":"image/png","content_base64":"bm90IHJlYWxseSBhbiBpbWFnZQ=="}}}"#,
        token,
    )
    .await;
    assert_eq!(fake.0, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(fake.1["error"]["data"]["code"], "unsupported_media_type");

    config.upload.max_size = 4;
    let oversized = perform_mcp_json_with_config(
        pool,
        config,
        r#"{"jsonrpc":"2.0","id":62,"method":"tools/call","params":{"name":"upload_image","arguments":{"filename":"huge.png","mime_type":"image/png","content_base64":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/lAbQwQAAAABJRU5ErkJggg=="}}}"#,
        token,
    )
    .await;
    assert_eq!(oversized.0, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(oversized.1["error"]["data"]["code"], "payload_too_large");
}

#[tokio::test]
async fn mcp_http_prompts_list_and_get_validate_arguments() {
    let pool = migrated_pool().await;
    let token = "prompt-token";
    insert_mcp_client(&pool, "prompter", token, &["blog.read"]).await;

    let list = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":70,"method":"prompts/list","params":{}}"#,
        token,
    )
    .await;
    assert_eq!(list.0, StatusCode::OK);
    let prompt_names = list.1["result"]["prompts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|prompt| prompt["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        prompt_names,
        vec![
            "draft_article_from_outline",
            "seo_review_article",
            "rewrite_article_summary"
        ]
    );

    let get = perform_mcp_json(
        pool.clone(),
        r##"{"jsonrpc":"2.0","id":71,"method":"prompts/get","params":{"name":"seo_review_article","arguments":{"title":"Rust SEO","content":"# Body","keywords":["rust","blog"]}}}"##,
        token,
    )
    .await;
    assert_eq!(get.0, StatusCode::OK);
    assert_eq!(get.1["result"]["name"], "seo_review_article");
    assert!(get.1["result"]["content"].as_str().unwrap().contains("SEO"));
    assert_eq!(get.1["result"]["input"]["title"], "Rust SEO");

    let invalid = perform_mcp_json(
        pool,
        r#"{"jsonrpc":"2.0","id":72,"method":"prompts/get","params":{"name":"rewrite_article_summary","arguments":{"title":"Bad","content":""}}}"#,
        token,
    )
    .await;
    assert_eq!(invalid.0, StatusCode::BAD_REQUEST);
    assert_eq!(invalid.1["error"]["data"]["code"], "invalid_params");
}

#[tokio::test]
async fn mcp_http_writes_audit_logs_for_success_and_denied_requests_without_raw_payload() {
    let pool = migrated_pool().await;
    let token = "audit-token";
    insert_mcp_client(&pool, "audited", token, &["blog.read"]).await;

    let success = perform_mcp_json(
        pool.clone(),
        r#"{"jsonrpc":"2.0","id":90,"method":"tools/list","params":{"note":"secret raw payload"}}"#,
        token,
    )
    .await;
    assert_eq!(success.0, StatusCode::OK);
    let row = sqlx::query(
        "SELECT transport, action_type, target, status, request_id, error_code, payload_digest
         FROM mcp_audit_logs ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<String, _>("transport"), "http");
    assert_eq!(row.get::<String, _>("action_type"), "tool_call");
    assert_eq!(row.get::<String, _>("target"), "tools/list");
    assert_eq!(row.get::<String, _>("status"), "success");
    assert_eq!(row.get::<String, _>("request_id"), "90");
    assert_eq!(row.get::<String, _>("error_code"), "");
    let digest: String = row.get("payload_digest");
    assert!(digest.starts_with("sha256:"));
    assert!(!digest.contains("secret raw payload"));

    let denied_response = mcp::router_with_pool_and_config(pool.clone(), Config::default())
        .oneshot(mcp_request(
            r#"{"jsonrpc":"2.0","id":91,"method":"initialize","params":{}}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(denied_response.status(), StatusCode::UNAUTHORIZED);

    let denied = sqlx::query(
        "SELECT client_id, transport, action_type, target, status, request_id, error_code
         FROM mcp_audit_logs ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denied.get::<Option<i64>, _>("client_id"), None);
    assert_eq!(denied.get::<String, _>("transport"), "http");
    assert_eq!(denied.get::<String, _>("action_type"), "tool_call");
    assert_eq!(denied.get::<String, _>("target"), "initialize");
    assert_eq!(denied.get::<String, _>("status"), "denied");
    assert_eq!(denied.get::<String, _>("request_id"), "91");
    assert_eq!(denied.get::<String, _>("error_code"), "auth_required");
}

#[tokio::test]
async fn mcp_http_rate_limit_applies_to_read_and_upload_buckets() {
    let pool = migrated_pool().await;
    let token = "rate-token";
    insert_mcp_client(&pool, "rate-client", token, &["blog.read", "blog.upload"]).await;
    let upload_dir = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.upload.dir = upload_dir.path().display().to_string();
    config.mcp.rate_limit.read_per_minute = 1;
    config.mcp.rate_limit.upload_per_10min = 1;
    let router = mcp::router_with_pool_and_config(pool, config);

    let first_read = router
        .clone()
        .oneshot(mcp_request(
            r#"{"jsonrpc":"2.0","id":100,"method":"tools/list","params":{}}"#,
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(first_read.status(), StatusCode::OK);
    let second_read = router
        .clone()
        .oneshot(mcp_request(
            r#"{"jsonrpc":"2.0","id":101,"method":"tools/list","params":{}}"#,
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(second_read.status(), StatusCode::TOO_MANY_REQUESTS);

    let upload_body = r#"{"jsonrpc":"2.0","id":102,"method":"tools/call","params":{"name":"upload_image","arguments":{"filename":"ok.png","mime_type":"image/png","content_base64":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/lAbQwQAAAABJRU5ErkJggg=="}}}"#;
    let first_upload = router
        .clone()
        .oneshot(mcp_request(upload_body, Some(token)))
        .await
        .unwrap();
    assert_eq!(first_upload.status(), StatusCode::OK);
    let second_upload = router
        .oneshot(mcp_request(upload_body, Some(token)))
        .await
        .unwrap();
    assert_eq!(second_upload.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = axum::body::to_bytes(second_upload.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["data"]["code"], "rate_limited");
}

#[test]
fn serve_mcp_http_fails_when_database_is_not_migrated_without_creating_db() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    let db_path = dir.path().join("blog.db");
    write_config(&config, &db_path);

    let output = Command::new(blogweb())
        .args(["serve-mcp", "-config"])
        .arg(&config)
        .args(["-transport", "http"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("schema_migrations"), "stderr: {stderr}");
    assert!(
        !db_path.exists(),
        "serve-mcp must not create or migrate the target database"
    );
}

#[test]
fn serve_mcp_stdio_processes_jsonrpc_until_eof_and_hides_write_tools_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.yaml");
    let db_path = dir.path().join("blog.db");
    write_config(&config, &db_path);

    let migrate = Command::new(blogweb())
        .args(["db", "migrate", "--apply", "-config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        migrate.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&migrate.stderr)
    );

    let mut child = Command::new(blogweb())
        .args(["serve-mcp", "-config"])
        .arg(&config)
        .args(["-transport", "stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(
                br##"{"jsonrpc":"2.0","id":80,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":81,"method":"tools/call","params":{"name":"preview_markdown","arguments":{"content":"# Hi"}}}
{"jsonrpc":"2.0","id":82,"method":"tools/call","params":{"name":"create_article_draft","arguments":{"title":"x","content":"# body"}}}
"##,
            )
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 3, "stdout: {stdout}");
    assert_eq!(lines[0]["id"], 80);
    assert!(!lines[0].to_string().contains("create_article_draft"));
    assert_eq!(lines[1]["result"]["content_html"], "<h1>Hi</h1>\n");
    assert_eq!(lines[2]["error"]["code"], 403);
    assert_eq!(lines[2]["error"]["data"]["code"], "forbidden_scope");
}

async fn migrated_pool() -> sqlx::Pool<sqlx::Sqlite> {
    let pool = db::connect_memory().await.unwrap();
    db::apply_migrations(&pool).await.unwrap();
    pool
}

async fn seeded_content_pool() -> sqlx::Pool<sqlx::Sqlite> {
    let pool = migrated_pool().await;
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
        "INSERT INTO articles (
            id, title, slug, content, cover_image, excerpt, category_id, author_id,
            status, is_pinned, published_at, created_at, updated_at
         ) VALUES
         (1, 'Rust Migration Baseline', 'rust-migration-baseline',
          '# Baseline\n\n<script>alert(1)</script>\n\nStable text.', '',
          'Baseline Stable text.', 1, 1, 'published', 0, '2026-05-29T08:00:00Z',
          '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z'),
         (2, 'Future Rust', 'future-rust', '# future', '', 'future', 1, 1,
          'published', 0, '2999-01-01T00:00:00Z',
          '2026-05-29T00:00:00Z', '2026-05-29T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn seeded_writer_pool() -> sqlx::Pool<sqlx::Sqlite> {
    let pool = migrated_pool().await;
    sqlx::query(
        "INSERT INTO users (id, username, password, role, created_at)
         VALUES
         (1, 'reader-one', 'hash', 'user', '2026-05-29T00:00:00Z'),
         (2, 'admin', 'hash', 'admin', '2026-05-29T00:00:00Z')",
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
    pool
}

async fn insert_mcp_client(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    name: &str,
    token: &str,
    scopes: &[&str],
) {
    sqlx::query(
        "INSERT INTO mcp_clients
         (name, token_hash, scopes, transport, is_enabled, created_at, updated_at)
         VALUES (?, ?, ?, 'http', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(name)
    .bind(hmac_sha256_hex(&Config::default().session.secret, token))
    .bind(serde_json::to_string(scopes).unwrap())
    .execute(pool)
    .await
    .unwrap();
}

async fn perform_mcp_json(
    pool: sqlx::Pool<sqlx::Sqlite>,
    body: impl Into<String>,
    token: &str,
) -> (StatusCode, Value) {
    perform_mcp_json_with_config(pool, Config::default(), body, token).await
}

async fn perform_mcp_json_with_config(
    pool: sqlx::Pool<sqlx::Sqlite>,
    config: Config,
    body: impl Into<String>,
    token: &str,
) -> (StatusCode, Value) {
    let response = mcp::router_with_pool_and_config(pool, config)
        .oneshot(mcp_request(body, Some(token)))
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

fn mcp_request(body: impl Into<String>, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Origin", "https://chatgpt.com");
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }
    builder.body(Body::from(body.into())).unwrap()
}

fn hmac_sha256_hex(secret: &str, value: &str) -> String {
    const BLOCK_SIZE: usize = 64;
    let mut key = secret.as_bytes().to_vec();
    if key.len() > BLOCK_SIZE {
        key = Sha256::digest(&key).to_vec();
    }
    key.resize(BLOCK_SIZE, 0);

    let mut ipad = [0x36; BLOCK_SIZE];
    let mut opad = [0x5c; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        ipad[index] ^= key[index];
        opad[index] ^= key[index];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(value.as_bytes());
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    to_hex(&outer.finalize())
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
