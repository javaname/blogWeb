use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Row, Sqlite,
};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

use crate::{
    db::DbPool,
    error::{AppError, Result},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    imported_rows: BTreeMap<&'static str, u64>,
}

impl SyncReport {
    pub fn imported_rows(&self, table: &str) -> u64 {
        self.imported_rows.get(table).copied().unwrap_or_default()
    }

    pub fn table_counts(&self) -> impl Iterator<Item = (&'static str, u64)> + '_ {
        self.imported_rows
            .iter()
            .map(|(table, count)| (*table, *count))
    }
}

#[derive(Clone, Copy)]
enum ColumnKind {
    Int,
    Text,
}

#[derive(Clone, Copy)]
struct ColumnSpec {
    name: &'static str,
    kind: ColumnKind,
}

#[derive(Clone, Copy)]
struct TableSpec {
    name: &'static str,
    columns: &'static [ColumnSpec],
}

const USERS: &[ColumnSpec] = &[
    int("id"),
    text("username"),
    text("password"),
    text("role"),
    text("created_at"),
    text("email"),
    text("email_verified_at"),
];
const CATEGORIES: &[ColumnSpec] = &[
    int("id"),
    text("name"),
    text("slug"),
    int("sort_order"),
    text("created_at"),
];
const ARTICLES: &[ColumnSpec] = &[
    int("id"),
    text("title"),
    text("slug"),
    text("content"),
    text("cover_image"),
    text("excerpt"),
    int("category_id"),
    int("author_id"),
    text("status"),
    int("is_pinned"),
    text("published_at"),
    text("created_at"),
    text("updated_at"),
];
const LIKES: &[ColumnSpec] = &[
    int("id"),
    int("article_id"),
    text("anonymous_id"),
    text("ip_address"),
    text("user_agent"),
    text("created_at"),
];
const SLUG_HISTORY: &[ColumnSpec] = &[
    int("id"),
    int("article_id"),
    text("old_slug"),
    text("created_at"),
];
const MCP_CLIENTS: &[ColumnSpec] = &[
    int("id"),
    text("name"),
    text("token_hash"),
    text("scopes"),
    text("transport"),
    int("is_enabled"),
    int("created_by"),
    text("last_used_at"),
    text("created_at"),
    text("updated_at"),
];
const MCP_AUDIT_LOGS: &[ColumnSpec] = &[
    int("id"),
    int("client_id"),
    text("transport"),
    text("action_type"),
    text("target"),
    text("scope"),
    text("status"),
    text("request_id"),
    text("actor_ip"),
    text("error_code"),
    text("payload_digest"),
    text("created_at"),
];
const COMMENTS: &[ColumnSpec] = &[
    int("id"),
    int("article_id"),
    text("author_name"),
    text("content"),
    text("status"),
    text("rejection_reason"),
    text("anonymous_id"),
    text("ip_address"),
    text("user_agent"),
    text("created_at"),
    text("updated_at"),
    int("parent_id"),
];
const NEWSLETTER_SUBSCRIPTIONS: &[ColumnSpec] = &[
    int("id"),
    text("email"),
    text("anonymous_id"),
    text("status"),
    text("ip_address"),
    text("user_agent"),
    text("created_at"),
    text("updated_at"),
];
const BOOKMARKS: &[ColumnSpec] = &[
    int("id"),
    int("article_id"),
    text("anonymous_id"),
    text("ip_address"),
    text("user_agent"),
    text("created_at"),
];
const AUTHOR_FOLLOWS: &[ColumnSpec] = &[
    int("id"),
    int("author_id"),
    text("anonymous_id"),
    text("ip_address"),
    text("user_agent"),
    text("created_at"),
];
const EMAIL_VERIFICATION_CODES: &[ColumnSpec] = &[
    int("id"),
    text("email"),
    text("code_hash"),
    text("expires_at"),
    text("used_at"),
    text("created_at"),
];

const TABLES: &[TableSpec] = &[
    table("users", USERS),
    table("categories", CATEGORIES),
    table("articles", ARTICLES),
    table("likes", LIKES),
    table("slug_history", SLUG_HISTORY),
    table("mcp_clients", MCP_CLIENTS),
    table("mcp_audit_logs", MCP_AUDIT_LOGS),
    table("comments", COMMENTS),
    table("newsletter_subscriptions", NEWSLETTER_SUBSCRIPTIONS),
    table("bookmarks", BOOKMARKS),
    table("author_follows", AUTHOR_FOLLOWS),
    table("email_verification_codes", EMAIL_VERIFICATION_CODES),
];

const fn int(name: &'static str) -> ColumnSpec {
    ColumnSpec {
        name,
        kind: ColumnKind::Int,
    }
}

const fn text(name: &'static str) -> ColumnSpec {
    ColumnSpec {
        name,
        kind: ColumnKind::Text,
    }
}

const fn table(name: &'static str, columns: &'static [ColumnSpec]) -> TableSpec {
    TableSpec { name, columns }
}

pub async fn sync_file(source_path: impl AsRef<Path>, target: &DbPool) -> Result<SyncReport> {
    let options = SqliteConnectOptions::new()
        .filename(source_path.as_ref())
        .create_if_missing(false);
    let source = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sync_pool(&source, target).await
}

pub async fn sync_pool(source: &Pool<Sqlite>, target: &DbPool) -> Result<SyncReport> {
    let mut report = SyncReport::default();
    for spec in TABLES {
        let count = import_table(source, target, spec).await?;
        report.imported_rows.insert(spec.name, count);
    }
    reset_sequences(target).await?;
    Ok(report)
}

async fn import_table(source: &Pool<Sqlite>, target: &DbPool, spec: &TableSpec) -> Result<u64> {
    if !sqlite_table_exists(source, spec.name).await? {
        return Ok(0);
    }
    let source_columns = sqlite_columns(source, spec.name).await?;
    let columns = spec
        .columns
        .iter()
        .copied()
        .filter(|column| source_columns.contains(column.name))
        .collect::<Vec<_>>();
    if columns.is_empty() || columns.iter().all(|column| column.name != "id") {
        return Err(AppError::Migration(format!(
            "source table {} does not contain importable id column",
            spec.name
        )));
    }

    let select_sql = format!(
        "SELECT {} FROM {} ORDER BY id ASC",
        columns
            .iter()
            .map(|column| sqlite_ident(column.name))
            .collect::<Result<Vec<_>>>()?
            .join(", "),
        sqlite_ident(spec.name)?,
    );
    let rows = sqlx::query(&select_sql).fetch_all(source).await?;
    let insert_sql = insert_sql(spec.name, &columns)?;
    for row in &rows {
        let mut query = sqlx::query(&insert_sql);
        for column in &columns {
            query = match column.kind {
                ColumnKind::Int => query.bind(row.try_get::<Option<i64>, _>(column.name)?),
                ColumnKind::Text => query.bind(row.try_get::<Option<String>, _>(column.name)?),
            };
        }
        query.execute(target).await?;
    }
    Ok(rows.len() as u64)
}

fn insert_sql(table: &str, columns: &[ColumnSpec]) -> Result<String> {
    let column_names = columns
        .iter()
        .map(|column| pg_ident(column.name))
        .collect::<Result<Vec<_>>>()?;
    let placeholders = (1..=columns.len())
        .map(|index| format!("${index}"))
        .collect::<Vec<_>>();
    let update_assignments = columns
        .iter()
        .filter(|column| column.name != "id")
        .map(|column| {
            let name = pg_ident(column.name)?;
            Ok(format!("{name} = EXCLUDED.{name}"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(format!(
        "INSERT INTO {} ({}) VALUES ({})
         ON CONFLICT(id) DO UPDATE SET {}",
        pg_ident(table)?,
        column_names.join(", "),
        placeholders.join(", "),
        update_assignments.join(", "),
    ))
}

async fn reset_sequences(target: &DbPool) -> Result<()> {
    for spec in TABLES {
        let sql = format!(
            "SELECT setval(
                pg_get_serial_sequence($1, 'id'),
                GREATEST(COALESCE(MAX(id), 0), 1),
                COALESCE(MAX(id), 0) > 0
             ) FROM {}",
            pg_ident(spec.name)?
        );
        sqlx::query(&sql).bind(spec.name).execute(target).await?;
    }
    Ok(())
}

async fn sqlite_table_exists(source: &Pool<Sqlite>, table: &str) -> Result<bool> {
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_optional(source)
            .await?;
    Ok(exists.is_some())
}

async fn sqlite_columns(source: &Pool<Sqlite>, table: &str) -> Result<HashSet<String>> {
    let sql = format!("PRAGMA table_info({})", sqlite_ident(table)?);
    let rows = sqlx::query(&sql).fetch_all(source).await?;
    let mut columns = HashSet::with_capacity(rows.len());
    for row in rows {
        columns.insert(row.try_get::<String, _>("name")?);
    }
    Ok(columns)
}

fn sqlite_ident(value: &str) -> Result<String> {
    ident(value, "sqlite")
}

fn pg_ident(value: &str) -> Result<String> {
    ident(value, "postgres")
}

fn ident(value: &str, dialect: &str) -> Result<String> {
    if value
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        Ok(format!("\"{}\"", value))
    } else {
        Err(AppError::Migration(format!(
            "invalid {dialect} identifier {value}"
        )))
    }
}
