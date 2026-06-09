use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgConnection, Pool, Postgres};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::error::{AppError, Result};

pub type Db = Postgres;
pub type DbPool = Pool<Db>;
pub type DbRow = sqlx::postgres::PgRow;

struct Migration {
    version: &'static str,
    filename: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: "001",
        filename: "001_init.sql",
        sql: include_str!("../migrations/001_init.sql"),
    },
    Migration {
        version: "002",
        filename: "002_mcp.sql",
        sql: include_str!("../migrations/002_mcp.sql"),
    },
    Migration {
        version: "003",
        filename: "003_comments.sql",
        sql: include_str!("../migrations/003_comments.sql"),
    },
    Migration {
        version: "004",
        filename: "004_reader_interactions.sql",
        sql: include_str!("../migrations/004_reader_interactions.sql"),
    },
    Migration {
        version: "005",
        filename: "005_email_registration.sql",
        sql: include_str!("../migrations/005_email_registration.sql"),
    },
    Migration {
        version: "006",
        filename: "006_role_permissions.sql",
        sql: include_str!("../migrations/006_role_permissions.sql"),
    },
];

const REQUIRED_SCHEMA: &[(&str, &[&str])] = &[
    (
        "users",
        &[
            "id",
            "username",
            "password",
            "role",
            "created_at",
            "email",
            "email_verified_at",
        ],
    ),
    (
        "categories",
        &["id", "name", "slug", "sort_order", "created_at"],
    ),
    (
        "articles",
        &[
            "id",
            "title",
            "slug",
            "content",
            "cover_image",
            "excerpt",
            "category_id",
            "author_id",
            "status",
            "is_pinned",
            "published_at",
            "created_at",
            "updated_at",
        ],
    ),
    ("likes", &["id", "article_id", "anonymous_id"]),
    ("slug_history", &["id", "article_id", "old_slug"]),
    (
        "mcp_clients",
        &[
            "id",
            "name",
            "token_hash",
            "scopes",
            "transport",
            "is_enabled",
        ],
    ),
    (
        "mcp_audit_logs",
        &["id", "client_id", "transport", "action_type", "status"],
    ),
    (
        "comments",
        &[
            "id",
            "article_id",
            "parent_id",
            "author_name",
            "content",
            "status",
        ],
    ),
    (
        "newsletter_subscriptions",
        &["id", "email", "anonymous_id", "status"],
    ),
    ("bookmarks", &["id", "article_id", "anonymous_id"]),
    ("author_follows", &["id", "author_id", "anonymous_id"]),
    (
        "email_verification_codes",
        &["id", "email", "code_hash", "expires_at", "used_at"],
    ),
    ("role_permissions", &["role", "permission", "created_at"]),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationCheckStatus {
    Ready,
    NeedsRegistration,
}

pub async fn connect(url: &str) -> Result<DbPool> {
    Ok(PgPoolOptions::new().max_connections(5).connect(url).await?)
}

pub async fn connect_existing(url: &str) -> Result<DbPool> {
    connect(url).await
}

pub async fn connect_memory() -> Result<DbPool> {
    let url = std::env::var("BLOGWEB_TEST_DATABASE_URL")
        .unwrap_or_else(|_| crate::config::DatabaseConfig::default().url);
    let schema = format!("test_{}", unique_suffix());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    sqlx::query(&format!("CREATE SCHEMA {}", pg_ident(&schema)?))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("SET search_path TO {}", pg_ident(&schema)?))
        .execute(&pool)
        .await?;
    Ok(pool)
}

pub async fn check_migrations(pool: &DbPool) -> Result<MigrationCheckStatus> {
    let has_table = table_exists(pool, "schema_migrations").await?;

    if has_table {
        for migration in MIGRATIONS {
            let recorded: Option<String> = sqlx::query_scalar(sql(
                "SELECT sha256 FROM schema_migrations WHERE version = ?",
            ))
            .bind(migration.version)
            .fetch_optional(pool)
            .await?;
            match recorded {
                Some(hash) if hash == migration_hash(migration.sql) => {}
                Some(_) => {
                    return Err(AppError::Migration(format!(
                        "migration hash mismatch for {}",
                        migration.filename
                    )));
                }
                None => {
                    return Err(AppError::Migration(format!(
                        "migration {} is not registered; run db migrate --apply",
                        migration.filename
                    )));
                }
            }
        }
        smoke_check_schema(pool).await?;
        Ok(MigrationCheckStatus::Ready)
    } else {
        Err(AppError::Migration(
            "schema_migrations table is missing; run db migrate --apply".into(),
        ))
    }
}

async fn smoke_check_schema(pool: &DbPool) -> Result<()> {
    for (table, columns) in REQUIRED_SCHEMA {
        if !table_exists(pool, table).await? {
            return Err(AppError::Migration(format!("missing table {table}")));
        }
        for column in *columns {
            if !column_exists(pool, table, column).await? {
                return Err(AppError::Migration(format!(
                    "missing column {table}.{column}"
                )));
            }
        }
    }
    Ok(())
}

async fn table_exists(pool: &DbPool, table: &str) -> Result<bool> {
    let exists: bool = sqlx::query_scalar(sql("SELECT EXISTS (
             SELECT 1
             FROM information_schema.tables
             WHERE table_schema = current_schema() AND table_name = ?
         )"))
    .bind(table)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn apply_migrations(pool: &DbPool) -> Result<()> {
    let mut conn = pool.acquire().await?;
    sqlx::query(crate::db::sql("BEGIN"))
        .execute(&mut *conn)
        .await?;
    let result = apply_migrations_in_transaction(&mut conn).await;
    match result {
        Ok(()) => {
            sqlx::query(crate::db::sql("COMMIT"))
                .execute(&mut *conn)
                .await?;
            Ok(())
        }
        Err(err) => {
            let _ = sqlx::query(crate::db::sql("ROLLBACK"))
                .execute(&mut *conn)
                .await;
            Err(err)
        }
    }
}

async fn apply_migrations_in_transaction(conn: &mut PgConnection) -> Result<()> {
    ensure_schema_migrations(conn).await?;

    for migration in MIGRATIONS {
        let expected_hash = migration_hash(migration.sql);
        let recorded: Option<String> = sqlx::query_scalar(sql(
            "SELECT sha256 FROM schema_migrations WHERE version = ?",
        ))
        .bind(migration.version)
        .fetch_optional(&mut *conn)
        .await?;
        match recorded {
            Some(hash) if hash == expected_hash => continue,
            Some(_) => {
                return Err(AppError::Migration(format!(
                    "migration hash mismatch for {}",
                    migration.filename
                )));
            }
            None => {
                execute_migration_sql(conn, migration.sql).await?;
                sqlx::query(sql(
                    "INSERT INTO schema_migrations (version, filename, sha256, applied_at)
                         VALUES (?, ?, ?, CURRENT_TIMESTAMP::text)",
                ))
                .bind(migration.version)
                .bind(migration.filename)
                .bind(expected_hash)
                .execute(&mut *conn)
                .await?;
            }
        }
    }

    verify_migration_hashes(conn).await?;
    smoke_check_schema_conn(conn).await?;
    Ok(())
}

async fn verify_migration_hashes(conn: &mut PgConnection) -> Result<()> {
    for migration in MIGRATIONS {
        let recorded: Option<String> = sqlx::query_scalar(sql(
            "SELECT sha256 FROM schema_migrations WHERE version = ?",
        ))
        .bind(migration.version)
        .fetch_optional(&mut *conn)
        .await?;
        match recorded {
            Some(hash) if hash == migration_hash(migration.sql) => {}
            Some(_) => {
                return Err(AppError::Migration(format!(
                    "migration hash mismatch for {}",
                    migration.filename
                )));
            }
            None => {
                return Err(AppError::Migration(format!(
                    "migration {} is not registered; run db migrate --apply",
                    migration.filename
                )));
            }
        }
    }
    Ok(())
}

async fn smoke_check_schema_conn(conn: &mut PgConnection) -> Result<()> {
    for (table, columns) in REQUIRED_SCHEMA {
        if !table_exists_conn(conn, table).await? {
            return Err(AppError::Migration(format!("missing table {table}")));
        }
        for column in *columns {
            if !column_exists_conn(conn, table, column).await? {
                return Err(AppError::Migration(format!(
                    "missing column {table}.{column}"
                )));
            }
        }
    }
    Ok(())
}

async fn table_exists_conn(conn: &mut PgConnection, table: &str) -> Result<bool> {
    let exists: bool = sqlx::query_scalar(sql("SELECT EXISTS (
             SELECT 1
             FROM information_schema.tables
             WHERE table_schema = current_schema() AND table_name = ?
         )"))
    .bind(table)
    .fetch_one(&mut *conn)
    .await?;
    Ok(exists)
}

async fn ensure_schema_migrations(conn: &mut PgConnection) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT NOT NULL PRIMARY KEY,
            filename TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )",
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn execute_migration_sql(conn: &mut PgConnection, migration_sql: &str) -> Result<()> {
    for statement in migration_sql.split(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        if let Some((table, column)) = parse_add_column(statement) {
            if column_exists_conn(conn, table, column).await? {
                continue;
            }
        }
        sqlx::query(statement).execute(&mut *conn).await?;
    }
    Ok(())
}

fn parse_add_column(statement: &str) -> Option<(&str, &str)> {
    let normalized = statement.split_whitespace().collect::<Vec<_>>();
    if normalized.len() < 6 {
        return None;
    }
    if !normalized[0].eq_ignore_ascii_case("ALTER")
        || !normalized[1].eq_ignore_ascii_case("TABLE")
        || !normalized[3].eq_ignore_ascii_case("ADD")
        || !normalized[4].eq_ignore_ascii_case("COLUMN")
    {
        return None;
    }
    normalized.get(5).map(|column| (normalized[2], *column))
}

async fn column_exists_conn(conn: &mut PgConnection, table: &str, column: &str) -> Result<bool> {
    let exists: bool = sqlx::query_scalar(sql("SELECT EXISTS (
             SELECT 1
             FROM information_schema.columns
             WHERE table_schema = current_schema()
               AND table_name = ?
               AND column_name = ?
         )"))
    .bind(table)
    .bind(column)
    .fetch_one(&mut *conn)
    .await?;
    Ok(exists)
}

async fn column_exists(pool: &DbPool, table: &str, column: &str) -> Result<bool> {
    let exists: bool = sqlx::query_scalar(sql("SELECT EXISTS (
             SELECT 1
             FROM information_schema.columns
             WHERE table_schema = current_schema()
               AND table_name = ?
               AND column_name = ?
         )"))
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

fn pg_ident(value: &str) -> Result<String> {
    if value
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        Ok(format!("\"{}\"", value))
    } else {
        Err(AppError::Migration(format!(
            "invalid postgres identifier {value}"
        )))
    }
}

pub fn sql(statement: &'static str) -> &'static str {
    if !statement.contains('?') {
        return statement;
    }
    static CACHE: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().expect("sql cache poisoned");
    if let Some(converted) = cache.get(statement) {
        return converted;
    }
    let converted = convert_placeholders(statement);
    let leaked = Box::leak(converted.into_boxed_str());
    cache.insert(statement, leaked);
    leaked
}

fn convert_placeholders(statement: &str) -> String {
    let mut converted = String::with_capacity(statement.len());
    let mut next = 1;
    for ch in statement.chars() {
        if ch == '?' {
            converted.push('$');
            converted.push_str(&next.to_string());
            next += 1;
        } else {
            converted.push(ch);
        }
    }
    converted
}

fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}_{}_{}", std::process::id(), counter, nanos)
}

fn migration_hash(sql: &str) -> String {
    format!("{:x}", Sha256::digest(sql.as_bytes()))
}
