use sha2::{Digest, Sha256};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite, SqliteConnection};

use crate::error::{AppError, Result};

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
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationCheckStatus {
    Ready,
    NeedsRegistration,
}

pub async fn connect(path: &str) -> Result<Pool<Sqlite>> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}

pub async fn connect_existing(path: &str) -> Result<Pool<Sqlite>> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false);
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}

pub async fn connect_memory() -> Result<Pool<Sqlite>> {
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?)
}

pub async fn check_migrations(pool: &Pool<Sqlite>) -> Result<MigrationCheckStatus> {
    let has_table: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
    )
    .fetch_optional(pool)
    .await?;

    if has_table.is_some() {
        for migration in MIGRATIONS {
            let recorded: Option<String> =
                sqlx::query_scalar("SELECT sha256 FROM schema_migrations WHERE version = ?")
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

async fn smoke_check_schema(pool: &Pool<Sqlite>) -> Result<()> {
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

async fn table_exists(pool: &Pool<Sqlite>, table: &str) -> Result<bool> {
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_optional(pool)
            .await?;
    Ok(exists.is_some())
}

pub async fn apply_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;
    let result = apply_migrations_in_transaction(&mut conn).await;
    match result {
        Ok(()) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(())
        }
        Err(err) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(err)
        }
    }
}

async fn apply_migrations_in_transaction(conn: &mut SqliteConnection) -> Result<()> {
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *conn)
        .await?;
    ensure_schema_migrations(conn).await?;

    for migration in MIGRATIONS {
        let expected_hash = migration_hash(migration.sql);
        let recorded: Option<String> =
            sqlx::query_scalar("SELECT sha256 FROM schema_migrations WHERE version = ?")
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
                sqlx::query(
                    "INSERT INTO schema_migrations (version, filename, sha256, applied_at)
                     VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
                )
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

async fn verify_migration_hashes(conn: &mut SqliteConnection) -> Result<()> {
    for migration in MIGRATIONS {
        let recorded: Option<String> =
            sqlx::query_scalar("SELECT sha256 FROM schema_migrations WHERE version = ?")
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

async fn smoke_check_schema_conn(conn: &mut SqliteConnection) -> Result<()> {
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

async fn table_exists_conn(conn: &mut SqliteConnection, table: &str) -> Result<bool> {
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(exists.is_some())
}

async fn ensure_schema_migrations(conn: &mut SqliteConnection) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT NOT NULL PRIMARY KEY,
            filename TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            applied_at DATETIME NOT NULL
        )",
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn execute_migration_sql(conn: &mut SqliteConnection, sql: &str) -> Result<()> {
    for statement in sql.split(';') {
        let statement = statement.trim();
        if statement.is_empty() || statement.eq_ignore_ascii_case("PRAGMA foreign_keys = ON") {
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

async fn column_exists_conn(
    conn: &mut SqliteConnection,
    table: &str,
    column: &str,
) -> Result<bool> {
    let pragma = format!("PRAGMA table_info({})", sqlite_ident(table)?);
    let rows = sqlx::query(&pragma).fetch_all(&mut *conn).await?;
    let mut names = Vec::with_capacity(rows.len());
    for row in rows {
        names.push(row.try_get::<String, _>("name")?);
    }
    Ok(names.iter().any(|name| name == column))
}

async fn column_exists(pool: &Pool<Sqlite>, table: &str, column: &str) -> Result<bool> {
    let pragma = format!("PRAGMA table_info({})", sqlite_ident(table)?);
    let rows = sqlx::query(&pragma).fetch_all(pool).await?;
    let mut names = Vec::with_capacity(rows.len());
    for row in rows {
        names.push(row.try_get::<String, _>("name")?);
    }
    Ok(names.iter().any(|name| name == column))
}

fn sqlite_ident(value: &str) -> Result<String> {
    if value
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        Ok(format!("\"{}\"", value))
    } else {
        Err(AppError::Migration(format!(
            "invalid sqlite identifier {value}"
        )))
    }
}

fn migration_hash(sql: &str) -> String {
    format!("{:x}", Sha256::digest(sql.as_bytes()))
}
