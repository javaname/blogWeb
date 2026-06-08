use axum::{
    extract::{Path, Query, State},
    http::{
        header::{CONTENT_TYPE, LOCATION},
        HeaderValue, StatusCode,
    },
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, QueryBuilder, Row, Sqlite};
use std::path::{Component, Path as FsPath, PathBuf};
use tokio::fs;

use crate::{
    config::Config,
    error::{AppError, Result},
    renderer,
};

#[derive(Clone)]
pub struct PublicState {
    pub db: Pool<Sqlite>,
    pub assets_dir: PathBuf,
    pub upload_dir: PathBuf,
    pub session_store: crate::session::RedisSessionStore,
    pub config: Config,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    limit: Option<i64>,
    category: Option<String>,
    keyword: Option<String>,
    cursor: Option<String>,
}

impl Default for ListQuery {
    fn default() -> Self {
        Self {
            limit: Some(12),
            category: None,
            keyword: None,
            cursor: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Cursor {
    is_pinned: i64,
    published_at: String,
    id: i64,
}

#[derive(Debug, Serialize)]
struct ListPublishedResult {
    list: Vec<PublicArticleSummary>,
    next_cursor: String,
    has_more: bool,
}

#[derive(Debug, Serialize)]
struct PublicArticleSummary {
    id: i64,
    title: String,
    slug: String,
    cover_image: String,
    excerpt: String,
    category: Option<PublicCategory>,
    author: PublicAuthor,
    is_pinned: bool,
    like_count: i64,
    read_time_min: i64,
    published_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct PublicArticleDetail {
    #[serde(flatten)]
    summary: PublicArticleSummary,
    content_html: String,
    user_liked: bool,
    user_bookmarked: bool,
    author_followed: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct PublicCategory {
    id: i64,
    name: String,
    slug: String,
}

#[derive(Debug)]
struct PublicCategoryWithCount {
    id: i64,
    name: String,
    slug: String,
    article_count: i64,
}

#[derive(Debug, Serialize)]
struct PublicAuthor {
    id: i64,
    username: String,
}

#[derive(Debug)]
struct PublicAuthorProfile {
    id: i64,
    username: String,
    article_count: i64,
    follower_count: i64,
}

#[derive(Debug)]
struct PublicCommentNode {
    id: i64,
    author_name: String,
    content: String,
    replies: Vec<PublicCommentNode>,
}

pub async fn home_page(
    State(state): State<PublicState>,
    Query(query): Query<ListQuery>,
) -> Result<axum::response::Response> {
    let keyword = query.keyword.clone().unwrap_or_default();
    let cursor = query.cursor.clone().unwrap_or_default();
    let categories = public_categories(&state).await?;
    let articles = published_summaries(
        &state,
        ListQuery {
            limit: Some(12),
            category: None,
            keyword: query.keyword,
            cursor: query.cursor,
        },
    )
    .await?;
    let mut list = articles.list;
    let hero = if keyword.trim().is_empty() && cursor.trim().is_empty() && !list.is_empty() {
        Some(list.remove(0))
    } else {
        None
    };
    Ok(html_response(render_article_list_page(
        &state.config,
        "home",
        "/",
        hero.as_ref(),
        &list,
        &categories,
        keyword.trim(),
        articles.has_more,
        &articles.next_cursor,
    )))
}

pub async fn search_page(
    State(state): State<PublicState>,
    Query(query): Query<ListQuery>,
) -> Result<axum::response::Response> {
    let keyword = query.keyword.clone().unwrap_or_default();
    let categories = public_categories(&state).await?;
    let articles = published_summaries(
        &state,
        ListQuery {
            limit: Some(12),
            category: None,
            keyword: query.keyword,
            cursor: query.cursor,
        },
    )
    .await?;
    Ok(html_response(render_article_list_page(
        &state.config,
        "search",
        "/search",
        None,
        &articles.list,
        &categories,
        keyword.trim(),
        articles.has_more,
        &articles.next_cursor,
    )))
}

pub async fn article_page(
    State(state): State<PublicState>,
    Path(slug): Path<String>,
) -> Result<axum::response::Response> {
    let detail = published_detail(&state.db, &slug).await?;
    let Some(detail) = detail else {
        return Err(AppError::HttpStatus(404, "not_found".into()));
    };
    let categories = public_categories(&state).await?;
    let comments = approved_comments(&state.db, detail.summary.id).await?;
    let related = related_articles(&state, &detail.summary).await?;
    Ok(html_response(render_article_page(
        &state.config,
        &detail,
        &comments,
        &related,
        &categories,
    )))
}

pub async fn category_page(
    State(state): State<PublicState>,
    Path(slug): Path<String>,
) -> Result<axum::response::Response> {
    let category = sqlx::query("SELECT id, name, slug FROM categories WHERE slug = ?")
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::HttpStatus(404, "not_found".into()))?;
    let category_id: i64 = category.try_get("id")?;
    let category_name: String = category.try_get("name")?;
    let category_slug: String = category.try_get("slug")?;
    let articles = published_summaries(
        &state,
        ListQuery {
            category: Some(category_slug.clone()),
            ..ListQuery::default()
        },
    )
    .await?;
    let categories = public_categories(&state).await?;
    let category = PublicCategory {
        id: category_id,
        name: category_name,
        slug: category_slug,
    };
    Ok(html_response(render_category_page(
        &state.config,
        &category,
        &articles.list,
        &categories,
    )))
}

pub async fn categories_index_page(
    State(state): State<PublicState>,
) -> Result<axum::response::Response> {
    let categories = public_categories(&state).await?;
    let total_articles: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM articles
         WHERE status = 'published'
           AND (published_at IS NULL OR published_at <= datetime('now'))",
    )
    .fetch_one(&state.db)
    .await?;
    Ok(html_response(render_categories_index_page(
        &state.config,
        &categories,
        total_articles,
    )))
}

pub async fn about_page(State(state): State<PublicState>) -> Result<axum::response::Response> {
    let categories = public_categories(&state).await?;
    Ok(html_response(render_about_page(&state.config, &categories)))
}

pub async fn author_page(
    State(state): State<PublicState>,
    Path(author_id): Path<i64>,
) -> Result<axum::response::Response> {
    let profile = public_author_profile(&state, author_id).await?;
    let Some(profile) = profile else {
        return Err(AppError::HttpStatus(404, "not_found".into()));
    };
    let articles = published_summaries_by_author(&state, author_id, 12).await?;
    let categories = public_categories(&state).await?;
    Ok(html_response(render_author_page(
        &state.config,
        &profile,
        &articles,
        &categories,
    )))
}

pub async fn tag_page(
    State(state): State<PublicState>,
    Path(slug): Path<String>,
) -> Result<axum::response::Response> {
    let label = tag_label(&slug);
    let categories = public_categories(&state).await?;
    let articles = published_summaries(
        &state,
        ListQuery {
            limit: Some(12),
            category: None,
            keyword: Some(label.clone()),
            cursor: None,
        },
    )
    .await?;
    Ok(html_response(render_tag_page(
        &state.config,
        &slug,
        &label,
        &articles.list,
        &categories,
    )))
}

pub async fn archive_page(State(state): State<PublicState>) -> Result<axum::response::Response> {
    let categories = public_categories(&state).await?;
    let articles = published_summaries(
        &state,
        ListQuery {
            limit: Some(50),
            category: None,
            keyword: None,
            cursor: None,
        },
    )
    .await?;
    Ok(html_response(render_archive_page(
        &state.config,
        &articles.list,
        &categories,
    )))
}

pub async fn not_found_page(State(state): State<PublicState>) -> axum::response::Response {
    let mut response = html_response(render_not_found_page(&state.config));
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

pub async fn serve_asset(
    State(state): State<PublicState>,
    Path(path): Path<String>,
) -> Result<axum::response::Response> {
    serve_file(state.assets_dir, path).await
}

pub async fn serve_upload(
    State(state): State<PublicState>,
    Path(path): Path<String>,
) -> Result<axum::response::Response> {
    serve_file(state.upload_dir, path).await
}

pub async fn list_articles(
    State(state): State<PublicState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::to_value(
        published_summaries(&state, query).await?,
    )?))
}

async fn published_summaries(state: &PublicState, query: ListQuery) -> Result<ListPublishedResult> {
    let limit = query.limit.unwrap_or(12).clamp(1, 50);
    let cursor = match query
        .cursor
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(value) => Some(serde_json::from_str::<Cursor>(value)?),
        None => None,
    };
    let mut builder = QueryBuilder::new(
        "SELECT
            articles.id,
            articles.title,
            articles.slug,
            articles.cover_image,
            articles.excerpt,
            articles.is_pinned,
            articles.published_at,
            categories.id AS category_id,
            categories.name AS category_name,
            categories.slug AS category_slug,
            users.id AS author_id,
            users.username AS author_username,
            COUNT(likes.id) AS like_count
         FROM articles
         LEFT JOIN categories ON categories.id = articles.category_id
         INNER JOIN users ON users.id = articles.author_id
         LEFT JOIN likes ON likes.article_id = articles.id
         WHERE articles.status = 'published'
           AND (articles.published_at IS NULL OR articles.published_at <= datetime('now'))",
    );
    if let Some(category) = query
        .category
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        builder.push(" AND categories.slug = ");
        builder.push_bind(category);
    }
    if let Some(keyword) = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        let pattern = format!("%{keyword}%");
        builder.push(" AND (articles.title LIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR articles.excerpt LIKE ");
        builder.push_bind(pattern);
        builder.push(")");
    }
    if let Some(cursor) = &cursor {
        builder.push(" AND (articles.is_pinned < ");
        builder.push_bind(cursor.is_pinned);
        builder.push(" OR (articles.is_pinned = ");
        builder.push_bind(cursor.is_pinned);
        builder.push(" AND articles.published_at < ");
        builder.push_bind(cursor.published_at.clone());
        builder.push(") OR (articles.is_pinned = ");
        builder.push_bind(cursor.is_pinned);
        builder.push(" AND articles.published_at = ");
        builder.push_bind(cursor.published_at.clone());
        builder.push(" AND articles.id < ");
        builder.push_bind(cursor.id);
        builder.push("))");
    }
    builder.push(
        " GROUP BY articles.id
          ORDER BY articles.is_pinned DESC, articles.published_at DESC, articles.id DESC
          LIMIT ",
    );
    builder.push_bind(limit + 1);
    let mut rows = builder.build().fetch_all(&state.db).await?;
    let has_more = rows.len() as i64 > limit;
    if has_more {
        rows.truncate(limit as usize);
    }

    let list = rows
        .into_iter()
        .map(|row| summary_from_row(&row))
        .collect::<Result<Vec<_>>>()?;
    let next_cursor = if has_more {
        list.last()
            .and_then(|item| {
                serde_json::to_string(&Cursor {
                    is_pinned: i64::from(item.is_pinned),
                    published_at: item.published_at.clone().unwrap_or_default(),
                    id: item.id,
                })
                .ok()
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    Ok(ListPublishedResult {
        list,
        next_cursor,
        has_more,
    })
}

pub async fn article_detail(
    State(state): State<PublicState>,
    Path(slug): Path<String>,
) -> Result<axum::response::Response> {
    let Some(detail) = published_detail(&state.db, &slug).await? else {
        if let Some(current_slug) = lookup_current_slug(&state.db, &slug).await? {
            let location = format!("/api/articles/{current_slug}");
            let mut response = StatusCode::MOVED_PERMANENTLY.into_response();
            response.headers_mut().insert(
                LOCATION,
                HeaderValue::from_str(&location)
                    .map_err(|err| AppError::Config(err.to_string()))?,
            );
            return Ok(response);
        }
        return Err(AppError::HttpStatus(404, "not_found".into()));
    };
    Ok(Json(serde_json::to_value(detail)?).into_response())
}

async fn published_detail(pool: &Pool<Sqlite>, slug: &str) -> Result<Option<PublicArticleDetail>> {
    let row = sqlx::query(
        "SELECT
            articles.id,
            articles.title,
            articles.slug,
            articles.content,
            articles.cover_image,
            articles.excerpt,
            articles.is_pinned,
            articles.published_at,
            articles.created_at,
            articles.updated_at,
            categories.id AS category_id,
            categories.name AS category_name,
            categories.slug AS category_slug,
            users.id AS author_id,
            users.username AS author_username,
            COUNT(likes.id) AS like_count
         FROM articles
         LEFT JOIN categories ON categories.id = articles.category_id
         INNER JOIN users ON users.id = articles.author_id
         LEFT JOIN likes ON likes.article_id = articles.id
         WHERE articles.slug = ?
           AND articles.status = 'published'
           AND (articles.published_at IS NULL OR articles.published_at <= datetime('now'))
         GROUP BY articles.id",
    )
    .bind(&slug)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    let summary = summary_from_row(&row)?;
    Ok(Some(PublicArticleDetail {
        summary,
        content_html: renderer::render_safe_html(&row.try_get::<String, _>("content")?)?.0,
        user_liked: false,
        user_bookmarked: false,
        author_followed: false,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    }))
}

async fn approved_comments(pool: &Pool<Sqlite>, article_id: i64) -> Result<Vec<PublicCommentNode>> {
    let rows = sqlx::query(
        "SELECT id, parent_id, author_name, content
         FROM comments
         WHERE article_id = ? AND status = 'approved'
         ORDER BY created_at ASC, id ASC",
    )
    .bind(article_id)
    .fetch_all(pool)
    .await?;
    let mut parents = Vec::<PublicCommentNode>::new();
    let mut replies = Vec::<(i64, PublicCommentNode)>::new();
    for row in rows {
        let node = PublicCommentNode {
            id: row.try_get("id")?,
            author_name: row.try_get("author_name")?,
            content: row.try_get("content")?,
            replies: Vec::new(),
        };
        match row.try_get::<Option<i64>, _>("parent_id")? {
            Some(parent_id) => replies.push((parent_id, node)),
            None => parents.push(node),
        }
    }
    for (parent_id, reply) in replies {
        if let Some(parent) = parents.iter_mut().find(|parent| parent.id == parent_id) {
            parent.replies.push(reply);
        }
    }
    parents.reverse();
    Ok(parents)
}

async fn related_articles(
    state: &PublicState,
    article: &PublicArticleSummary,
) -> Result<Vec<PublicArticleSummary>> {
    let Some(category) = &article.category else {
        return Ok(Vec::new());
    };
    let rows = sqlx::query(
        "SELECT
            articles.id,
            articles.title,
            articles.slug,
            articles.cover_image,
            articles.excerpt,
            articles.is_pinned,
            articles.published_at,
            categories.id AS category_id,
            categories.name AS category_name,
            categories.slug AS category_slug,
            users.id AS author_id,
            users.username AS author_username,
            COUNT(likes.id) AS like_count
         FROM articles
         LEFT JOIN categories ON categories.id = articles.category_id
         INNER JOIN users ON users.id = articles.author_id
         LEFT JOIN likes ON likes.article_id = articles.id
         WHERE articles.id <> ?
           AND articles.category_id = ?
           AND articles.status = 'published'
           AND (articles.published_at IS NULL OR articles.published_at <= datetime('now'))
         GROUP BY articles.id
         ORDER BY articles.published_at DESC, articles.id DESC
         LIMIT 3",
    )
    .bind(article.id)
    .bind(category.id)
    .fetch_all(&state.db)
    .await?;
    rows.into_iter()
        .map(|row| summary_from_row(&row))
        .collect::<Result<Vec<_>>>()
}

async fn lookup_current_slug(pool: &Pool<Sqlite>, old_slug: &str) -> Result<Option<String>> {
    let slug = sqlx::query_scalar(
        "SELECT articles.slug
         FROM slug_history
         INNER JOIN articles ON articles.id = slug_history.article_id
         WHERE slug_history.old_slug = ?",
    )
    .bind(old_slug)
    .fetch_optional(pool)
    .await?;
    Ok(slug)
}

async fn public_categories(state: &PublicState) -> Result<Vec<PublicCategoryWithCount>> {
    let rows = sqlx::query(
        "SELECT
            categories.id,
            categories.name,
            categories.slug,
            COUNT(articles.id) AS article_count
         FROM categories
         LEFT JOIN articles ON articles.category_id = categories.id
         GROUP BY categories.id
         ORDER BY categories.sort_order ASC, categories.id ASC",
    )
    .fetch_all(&state.db)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(PublicCategoryWithCount {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                article_count: row.try_get("article_count")?,
            })
        })
        .collect()
}

async fn public_author_profile(
    state: &PublicState,
    author_id: i64,
) -> Result<Option<PublicAuthorProfile>> {
    let row = sqlx::query(
        "SELECT
            users.id,
            users.username,
            COUNT(DISTINCT articles.id) AS article_count,
            COUNT(DISTINCT author_follows.id) AS follower_count
         FROM users
         LEFT JOIN articles ON articles.author_id = users.id
            AND articles.status = 'published'
            AND (articles.published_at IS NULL OR articles.published_at <= datetime('now'))
         LEFT JOIN author_follows ON author_follows.author_id = users.id
         WHERE users.id = ?
         GROUP BY users.id",
    )
    .bind(author_id)
    .fetch_optional(&state.db)
    .await?;
    row.map(|row| {
        Ok(PublicAuthorProfile {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            article_count: row.try_get("article_count")?,
            follower_count: row.try_get("follower_count")?,
        })
    })
    .transpose()
}

async fn published_summaries_by_author(
    state: &PublicState,
    author_id: i64,
    limit: i64,
) -> Result<Vec<PublicArticleSummary>> {
    let rows = sqlx::query(
        "SELECT
            articles.id,
            articles.title,
            articles.slug,
            articles.cover_image,
            articles.excerpt,
            articles.is_pinned,
            articles.published_at,
            categories.id AS category_id,
            categories.name AS category_name,
            categories.slug AS category_slug,
            users.id AS author_id,
            users.username AS author_username,
            COUNT(likes.id) AS like_count
         FROM articles
         LEFT JOIN categories ON categories.id = articles.category_id
         INNER JOIN users ON users.id = articles.author_id
         LEFT JOIN likes ON likes.article_id = articles.id
         WHERE articles.author_id = ?
           AND articles.status = 'published'
           AND (articles.published_at IS NULL OR articles.published_at <= datetime('now'))
         GROUP BY articles.id
         ORDER BY articles.is_pinned DESC, articles.published_at DESC, articles.id DESC
         LIMIT ?",
    )
    .bind(author_id)
    .bind(limit.clamp(1, 50))
    .fetch_all(&state.db)
    .await?;
    rows.into_iter().map(|row| summary_from_row(&row)).collect()
}

fn summary_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<PublicArticleSummary> {
    let category_id: Option<i64> = row.try_get("category_id")?;
    let category = match category_id {
        Some(id) => Some(PublicCategory {
            id,
            name: row.try_get("category_name")?,
            slug: row.try_get("category_slug")?,
        }),
        None => None,
    };
    let content: Option<String> = row.try_get("content").ok();
    let excerpt: String = row.try_get("excerpt")?;
    Ok(PublicArticleSummary {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        slug: row.try_get("slug")?,
        cover_image: row.try_get("cover_image")?,
        excerpt,
        category,
        author: PublicAuthor {
            id: row.try_get("author_id")?,
            username: row.try_get("author_username")?,
        },
        is_pinned: row.try_get::<i64, _>("is_pinned")? != 0,
        like_count: row.try_get("like_count")?,
        read_time_min: content.as_deref().map(read_time_min).unwrap_or(1),
        published_at: row.try_get("published_at")?,
    })
}

fn read_time_min(content: &str) -> i64 {
    let words = content.split_whitespace().count() as i64;
    (words / 300).max(1)
}

fn render_article_list_page(
    config: &Config,
    page: &str,
    list_path: &str,
    hero: Option<&PublicArticleSummary>,
    articles: &[PublicArticleSummary],
    categories: &[PublicCategoryWithCount],
    keyword: &str,
    has_more: bool,
    next_cursor: &str,
) -> String {
    let mut html = document_start(
        page,
        &config.site.title,
        &config.site.description,
        "bg-background text-on-surface",
        false,
    );
    html.push_str(&topnav(config, keyword));
    html.push_str("<main class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop pt-10 pb-20\">");
    if let Some(hero) = hero {
        html.push_str("<section class=\"mb-20\"><a href=\"/articles/");
        html.push_str(&escape_html(&hero.slug));
        html.push_str("\" class=\"block group cursor-pointer\" data-article-slug=\"");
        html.push_str(&escape_html(&hero.slug));
        html.push_str("\"><div class=\"relative overflow-hidden rounded-xl bg-surface-container-low border border-outline-variant transition-all hover:shadow-lg\"><div class=\"grid md:grid-cols-2 gap-0\"><div class=\"relative aspect-[4/3] md:aspect-auto overflow-hidden bg-surface-container-high\">");
        render_article_image(
            &mut html,
            hero,
            "absolute inset-0 w-full h-full object-cover",
        );
        if hero.is_pinned {
            html.push_str("<div class=\"absolute top-4 left-4\"><span class=\"bg-primary-container text-on-primary-container px-3 py-1 rounded-full text-label-sm font-label-sm\">精选</span></div>");
        }
        html.push_str("</div><div class=\"p-8 md:p-12 flex flex-col justify-center\"><div class=\"flex items-center gap-3 mb-6\">");
        render_category_label(&mut html, hero.category.as_ref());
        html.push_str("<span class=\"text-on-surface-variant text-label-sm font-label-sm\">");
        html.push_str(&format_date(hero.published_at.as_deref()));
        html.push_str("</span></div><h2 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg mb-6 leading-tight group-hover:text-primary transition-colors\">");
        html.push_str(&escape_html(&hero.title));
        html.push_str("</h2><p class=\"font-interface-md text-interface-md text-on-surface-variant mb-8 line-clamp-3\">");
        html.push_str(&escape_html(&hero.excerpt));
        html.push_str("</p><div class=\"flex items-center gap-4\"><div class=\"w-10 h-10 rounded-full bg-primary text-on-primary flex items-center justify-center font-bold\">");
        html.push_str(&author_initial(&hero.author.username));
        html.push_str("</div><div><p class=\"font-interface-md text-interface-md font-bold\">");
        html.push_str(&author_name(&hero.author.username));
        html.push_str("</p><p class=\"text-caption font-caption text-on-surface-variant\">");
        html.push_str(&hero.read_time_min.to_string());
        html.push_str(" 分钟阅读</p></div></div><button class=\"like-button hidden\" type=\"button\" data-like-button data-slug=\"");
        html.push_str(&escape_html(&hero.slug));
        html.push_str("\" aria-label=\"喜欢文章\"><span class=\"like-count\">");
        html.push_str(&hero.like_count.to_string());
        html.push_str("</span></button></div></div></div></a></section>");
    }
    html.push_str("<div class=\"grid grid-cols-1 lg:grid-cols-12 gap-12\"><div class=\"lg:col-span-8 space-y-12\"><div class=\"flex items-center justify-between border-b border-outline-variant pb-4\"><div><h2 class=\"font-headline-md text-headline-md text-on-surface\">");
    html.push_str(if keyword.is_empty() {
        "最新文章"
    } else {
        "搜索结果"
    });
    html.push_str("</h2>");
    if !keyword.is_empty() {
        html.push_str(
            "<p class=\"font-caption text-caption text-on-surface-variant mt-2\">关键词：",
        );
        html.push_str(&escape_html(keyword));
        html.push_str("</p>");
    }
    html.push_str("</div>");
    if !keyword.is_empty() {
        html.push_str("<a class=\"font-caption text-caption text-primary hover:underline\" href=\"/search\">清除搜索</a>");
    }
    html.push_str("</div>");
    render_article_grid(&mut html, articles);
    if articles.is_empty() && hero.is_none() {
        html.push_str("<div class=\"text-center py-20\"><span class=\"material-symbols-outlined text-[64px] text-on-surface-variant opacity-40\">article</span><p class=\"font-interface-md text-interface-md text-on-surface-variant mt-4\">");
        html.push_str(if keyword.is_empty() {
            "暂无已发布文章，请稍后再来。"
        } else {
            "没有找到匹配的文章。"
        });
        html.push_str("</p></div>");
    }
    if has_more && !next_cursor.is_empty() {
        html.push_str("<div class=\"flex items-center justify-center gap-4 pt-8\"><a href=\"");
        html.push_str(list_path);
        html.push_str("?cursor=");
        html.push_str(&escape_html(next_cursor));
        if !keyword.is_empty() {
            html.push_str("&keyword=");
            html.push_str(&url_encode(keyword));
        }
        html.push_str("\" class=\"px-6 py-2 rounded-lg border border-outline-variant hover:bg-surface-container transition-colors font-interface-md text-interface-md flex items-center gap-2\">更早文章 <span class=\"material-symbols-outlined text-[20px]\">chevron_right</span></a></div>");
    }
    html.push_str("</div><aside class=\"lg:col-span-4 space-y-8\">");
    html.push_str(&sidebar_newsletter());
    html.push_str(&sidebar_categories(categories, None));
    html.push_str("</aside></div></main>");
    html.push_str(&site_footer(config));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn render_article_page(
    config: &Config,
    detail: &PublicArticleDetail,
    comments: &[PublicCommentNode],
    related: &[PublicArticleSummary],
    categories: &[PublicCategoryWithCount],
) -> String {
    let title = format!("{} &mdash; {}", detail.summary.title, config.site.title);
    let mut html = document_start(
        "article",
        &title,
        &detail.summary.excerpt,
        "bg-surface-container-lowest text-on-surface antialiased",
        true,
    );
    html.push_str("<div class=\"reading-progress\" id=\"reading-progress\"></div>");
    html.push_str(&topnav(config, ""));
    html.push_str("<main class=\"w-full\"><header class=\"w-full bg-surface-container-low pt-12 md:pt-20 pb-16\"><div class=\"max-w-article-max mx-auto px-margin-mobile md:px-0\"><div class=\"flex flex-wrap gap-2 mb-6\">");
    render_category_pill(&mut html, detail.summary.category.as_ref());
    if detail.summary.is_pinned {
        html.push_str("<span class=\"bg-primary-container text-on-primary-container px-3 py-1 rounded-full text-caption font-interface-md uppercase tracking-wider\">精选</span>");
    }
    html.push_str("</div><h1 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg text-on-surface mb-8 leading-tight\">");
    html.push_str(&escape_html(&detail.summary.title));
    html.push_str("</h1><div class=\"flex items-center gap-4 mb-12 flex-wrap\"><div class=\"w-12 h-12 rounded-full overflow-hidden bg-outline-variant\"><img class=\"w-full h-full object-cover\" src=\"");
    html.push_str(&author_avatar(&detail.summary.author.username));
    html.push_str("\" alt=\"");
    html.push_str(&author_name(&detail.summary.author.username));
    html.push_str(
        "\"></div><div><p class=\"font-interface-md text-interface-md text-on-surface\">",
    );
    html.push_str(&author_name(&detail.summary.author.username));
    html.push_str("</p><p class=\"font-caption text-caption text-on-surface-variant\">");
    html.push_str(&format_date(detail.summary.published_at.as_deref()));
    html.push_str(" &middot; ");
    html.push_str(&detail.summary.read_time_min.to_string());
    html.push_str(" 分钟阅读</p></div><div class=\"ml-auto flex gap-3 items-center\">");
    html.push_str("<button class=\"like-button flex items-center gap-1 px-3 py-2 rounded-full text-on-surface-variant hover:text-error hover:bg-surface-container-high transition-colors");
    if detail.user_liked {
        html.push_str(" text-error");
    }
    html.push_str("\" type=\"button\" data-like-button data-slug=\"");
    html.push_str(&escape_html(&detail.summary.slug));
    html.push_str(
        "\" aria-label=\"喜欢文章\"><span class=\"material-symbols-outlined like-icon\">",
    );
    html.push_str(if detail.user_liked {
        "favorite"
    } else {
        "favorite_border"
    });
    html.push_str("</span><span class=\"like-count font-interface-md text-interface-md\">");
    html.push_str(&detail.summary.like_count.to_string());
    html.push_str("</span></button><button type=\"button\" class=\"p-2 text-on-surface-variant hover:bg-surface-container-high rounded-full transition-colors\" aria-label=\"分享\" data-share-page><span class=\"material-symbols-outlined\">share</span></button>");
    html.push_str("<button type=\"button\" class=\"p-2 hover:bg-surface-container-high rounded-full transition-colors");
    if detail.user_bookmarked {
        html.push_str(" text-primary bookmarked");
    } else {
        html.push_str(" text-on-surface-variant");
    }
    html.push_str("\" aria-label=\"收藏\" data-bookmark-button data-slug=\"");
    html.push_str(&escape_html(&detail.summary.slug));
    html.push_str("\"><span class=\"material-symbols-outlined bookmark-icon\">");
    html.push_str(if detail.user_bookmarked {
        "bookmark"
    } else {
        "bookmark_border"
    });
    html.push_str("</span></button></div></div></div>");
    if !detail.summary.cover_image.is_empty() {
        html.push_str("<div class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop\"><div class=\"aspect-[21/9] w-full rounded-xl overflow-hidden shadow-sm bg-surface-container-high\"><img class=\"w-full h-full object-cover\" src=\"");
        html.push_str(&escape_html(&detail.summary.cover_image));
        html.push_str("\" alt=\"");
        html.push_str(&escape_html(&detail.summary.title));
        html.push_str("\"></div></div>");
    }
    html.push_str("</header><article class=\"max-w-article-max mx-auto px-margin-mobile md:px-0 py-16\"><div class=\"font-article-body-mobile md:font-article-body text-article-body-mobile md:text-article-body text-on-surface\"><div class=\"article-html\">");
    html.push_str(&detail.content_html);
    html.push_str("</div></div><section class=\"mt-20 pt-12 border-t border-outline-variant\"><div class=\"flex flex-col md:flex-row gap-8 items-center bg-surface-container-low p-8 rounded-xl\"><img class=\"w-24 h-24 rounded-full shrink-0 object-cover\" src=\"");
    html.push_str(&author_avatar(&detail.summary.author.username));
    html.push_str("\" alt=\"");
    html.push_str(&author_name(&detail.summary.author.username));
    html.push_str("\"><div class=\"text-center md:text-left\"><h4 class=\"font-headline-md text-headline-md mb-2\">作者：");
    html.push_str(&author_name(&detail.summary.author.username));
    html.push_str("</h4><p class=\"text-on-surface-variant mb-4\">");
    html.push_str(&author_bio(&detail.summary.author.username));
    html.push_str("</p><div class=\"flex justify-center md:justify-start gap-4\"><button class=\"text-primary font-bold hover:underline\" type=\"button\" data-follow-author data-author-id=\"");
    html.push_str(&detail.summary.author.id.to_string());
    html.push_str("\" data-author-name=\"");
    html.push_str(&author_name(&detail.summary.author.username));
    html.push_str("\" data-following=\"");
    html.push_str(if detail.author_followed {
        "true\">已关注 "
    } else {
        "false\">关注 "
    });
    html.push_str(&author_name(&detail.summary.author.username));
    html.push_str("</button></div></div></div></section></article>");
    html.push_str("<section class=\"bg-surface-container-low py-20\"><div class=\"max-w-article-max mx-auto px-margin-mobile md:px-0\"><h3 class=\"font-headline-md text-headline-md mb-8 flex items-center gap-3\">评论 <span class=\"bg-primary-container text-on-primary-container px-2 py-0.5 rounded text-caption\">");
    html.push_str(&comments.len().to_string());
    html.push_str("</span></h3>");
    html.push_str(&comment_form(&detail.summary.slug));
    render_comments(&mut html, comments);
    html.push_str("</div></section>");
    if !related.is_empty() {
        html.push_str("<section class=\"py-20 bg-surface-container-lowest\"><div class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop\"><h3 class=\"font-headline-md text-headline-md mb-12\">相关文章</h3>");
        render_related_grid(&mut html, related);
        html.push_str("</div></section>");
    }
    html.push_str("</main>");
    html.push_str(&site_footer(config));
    html.push_str(&hidden_sidebar_fallback(categories));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn render_category_page(
    config: &Config,
    category: &PublicCategory,
    articles: &[PublicArticleSummary],
    categories: &[PublicCategoryWithCount],
) -> String {
    let title = format!("{} &mdash; {}", category.name, config.site.title);
    let mut html = document_start(
        "category",
        &title,
        &config.site.description,
        "bg-background text-on-surface",
        false,
    );
    html.push_str(&topnav(config, ""));
    html.push_str("<main class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop pt-10 pb-20\"><section class=\"mb-12 pb-8 border-b border-outline-variant\"><p class=\"text-primary font-label-sm text-label-sm uppercase tracking-wider mb-3\">分类</p><h1 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg leading-tight mb-4\">");
    html.push_str(&escape_html(&category.name));
    html.push_str(
        "</h1><p class=\"font-interface-md text-interface-md text-on-surface-variant\">收录于「",
    );
    html.push_str(&escape_html(&category.name));
    html.push_str("」分类下的精选文章。</p></section><div class=\"grid grid-cols-1 lg:grid-cols-12 gap-12\"><div class=\"lg:col-span-8 space-y-12\"><div class=\"flex items-center justify-between\"><h2 class=\"font-headline-md text-headline-md text-on-surface\">");
    if articles.is_empty() {
        html.push_str("暂无文章");
    } else {
        html.push_str(&articles.len().to_string());
        html.push_str(" 篇文章");
    }
    html.push_str("</h2></div>");
    render_article_grid(&mut html, articles);
    if articles.is_empty() {
        html.push_str("<div class=\"text-center py-20\"><span class=\"material-symbols-outlined text-[64px] text-on-surface-variant opacity-40\">article</span><p class=\"font-interface-md text-interface-md text-on-surface-variant mt-4\">该分类下暂无文章。</p></div>");
    }
    html.push_str("</div><aside class=\"lg:col-span-4 space-y-8\">");
    html.push_str(&sidebar_newsletter());
    html.push_str(&sidebar_categories(categories, Some(&category.slug)));
    html.push_str("</aside></div></main>");
    html.push_str(&site_footer(config));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn render_categories_index_page(
    config: &Config,
    categories: &[PublicCategoryWithCount],
    total_articles: i64,
) -> String {
    let mut html = document_start(
        "categories",
        &format!("分类浏览 &mdash; {}", config.site.title),
        &config.site.description,
        "bg-background text-on-surface",
        false,
    );
    html.push_str(&topnav(config, ""));
    html.push_str("<main class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop pt-10 pb-20\"><section class=\"mb-12 pb-8 border-b border-outline-variant\"><p class=\"text-primary font-label-sm text-label-sm uppercase tracking-wider mb-3\">分类浏览</p><h1 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg leading-tight mb-4\">探索主题</h1><p class=\"font-interface-md text-interface-md text-on-surface-variant max-w-[720px]\">按照主题浏览所有公开文章，快速找到技术、设计和创作方法相关内容。</p><div class=\"mt-8 grid grid-cols-1 sm:grid-cols-3 gap-4\"><article class=\"rounded-xl border border-outline-variant bg-surface-container-low p-5\"><span class=\"material-symbols-outlined text-primary\">category</span><strong class=\"block mt-3 text-headline-md font-headline-md\">");
    html.push_str(&categories.len().to_string());
    html.push_str(" 个分类</strong></article><article class=\"rounded-xl border border-outline-variant bg-surface-container-low p-5\"><span class=\"material-symbols-outlined text-primary\">article</span><strong class=\"block mt-3 text-headline-md font-headline-md\">");
    html.push_str(&total_articles.to_string());
    html.push_str(" 篇文章</strong></article><article class=\"rounded-xl border border-outline-variant bg-surface-container-low p-5\"><span class=\"material-symbols-outlined text-primary\">sell</span><strong class=\"block mt-3 text-headline-md font-headline-md\">精选主题</strong></article></div></section><section class=\"grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8\">");
    for category in categories {
        html.push_str("<a href=\"/categories/");
        html.push_str(&escape_html(&category.slug));
        html.push_str("\" class=\"group block rounded-xl border border-outline-variant bg-surface-container-low p-6 hover:shadow-lg transition-all\" data-category-id=\"");
        html.push_str(&category.id.to_string());
        html.push_str("\"><div class=\"flex items-center justify-between mb-6\"><span class=\"material-symbols-outlined text-primary\">folder</span><span class=\"text-caption font-caption bg-surface-container px-3 py-1 rounded-full text-on-surface-variant\">");
        html.push_str(&category.article_count.to_string());
        html.push_str(if category.article_count == 1 {
            " 篇文章"
        } else {
            " 篇文章"
        });
        html.push_str("</span></div><h2 class=\"font-headline-md text-headline-md mb-3 group-hover:text-primary transition-colors\">");
        html.push_str(&escape_html(&category.name));
        html.push_str(
            "</h2><p class=\"font-interface-md text-interface-md text-on-surface-variant mb-6\">",
        );
        html.push_str(&category_description(&category.slug, &category.name));
        html.push_str("</p><span class=\"font-interface-md text-interface-md text-primary font-bold\">查看文章 <span class=\"material-symbols-outlined text-[18px] align-middle\">arrow_forward</span></span></a>");
    }
    html.push_str("</section></main>");
    html.push_str(&site_footer(config));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn render_about_page(config: &Config, categories: &[PublicCategoryWithCount]) -> String {
    let mut html = document_start(
        "about",
        &format!("关于我们 &mdash; {}", config.site.title),
        &config.site.description,
        "bg-background text-on-surface",
        false,
    );
    html.push_str(&topnav(config, ""));
    html.push_str("<main class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop pt-10 pb-20\"><section class=\"grid grid-cols-1 lg:grid-cols-12 gap-12 items-start mb-20\"><div class=\"lg:col-span-8\"><p class=\"text-primary font-label-sm text-label-sm uppercase tracking-wider mb-4\">关于</p><h1 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg leading-tight mb-6\">关于 ");
    html.push_str(&escape_html(&config.site.title));
    html.push_str("</h1><p class=\"font-interface-md text-interface-md text-on-surface-variant max-w-[760px]\">");
    html.push_str(&escape_html(&config.site.description));
    html.push_str("。我们关注现代设计、工程实践和长期主义创作，强调清晰、克制和可执行的思考。</p></div><aside class=\"lg:col-span-4\">");
    html.push_str(&sidebar_newsletter());
    html.push_str("</aside></section><section class=\"mb-20\"><h2 class=\"font-headline-md text-headline-md mb-8\">编辑原则</h2><div class=\"grid grid-cols-1 md:grid-cols-3 gap-6\"><article class=\"rounded-xl border border-outline-variant bg-surface-container-low p-6\"><span class=\"material-symbols-outlined text-primary\">manage_search</span><h3 class=\"font-interface-md text-interface-md font-bold mt-4 mb-3\">深度优先</h3><p class=\"text-on-surface-variant\">优先发表经过验证、结构完整、能帮助读者做判断的内容。</p></article><article class=\"rounded-xl border border-outline-variant bg-surface-container-low p-6\"><span class=\"material-symbols-outlined text-primary\">format_quote</span><h3 class=\"font-interface-md text-interface-md font-bold mt-4 mb-3\">清晰表达</h3><p class=\"text-on-surface-variant\">用明确语言解释复杂主题，减少术语噪声和不必要的包装。</p></article><article class=\"rounded-xl border border-outline-variant bg-surface-container-low p-6\"><span class=\"material-symbols-outlined text-primary\">verified</span><h3 class=\"font-interface-md text-interface-md font-bold mt-4 mb-3\">实践校验</h3><p class=\"text-on-surface-variant\">把设计观点、技术选择和产品经验落到可复用的方法中。</p></article></div></section><section class=\"grid grid-cols-1 lg:grid-cols-12 gap-12\"><div class=\"lg:col-span-8\"><h2 class=\"font-headline-md text-headline-md mb-6\">主题范围</h2>");
    html.push_str(&sidebar_categories(categories, None));
    html.push_str("</div><div class=\"lg:col-span-4 rounded-xl border border-outline-variant bg-surface-container-low p-6\"><h2 class=\"font-headline-md text-headline-md mb-4\">合作方式</h2><p class=\"text-on-surface-variant mb-6\">欢迎围绕内容系统、产品工程、设计实践和写作流程交流。</p><a href=\"/categories\" class=\"inline-flex items-center gap-2 text-primary font-bold\">浏览全部主题 <span class=\"material-symbols-outlined text-[18px]\">arrow_forward</span></a></div></section></main>");
    html.push_str(&site_footer(config));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn render_author_page(
    config: &Config,
    profile: &PublicAuthorProfile,
    articles: &[PublicArticleSummary],
    categories: &[PublicCategoryWithCount],
) -> String {
    let display_name = author_name(&profile.username);
    let mut html = document_start(
        "author",
        &format!("{} &mdash; {}", display_name, config.site.title),
        &author_bio(&profile.username),
        "bg-background text-on-surface",
        false,
    );
    html.push_str(&topnav(config, ""));
    html.push_str("<main class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop pt-10 pb-20\"><section class=\"rounded-xl border border-outline-variant bg-surface-container-low p-8 md:p-10 mb-12\"><div class=\"flex flex-col md:flex-row gap-8 items-start\"><img class=\"w-24 h-24 rounded-full object-cover bg-outline-variant\" src=\"");
    html.push_str(&author_avatar(&profile.username));
    html.push_str("\" alt=\"");
    html.push_str(&escape_html(&display_name));
    html.push_str("\"><div class=\"flex-1\"><p class=\"text-primary font-label-sm text-label-sm uppercase tracking-wider mb-3\">作者主页</p><h1 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg leading-tight mb-4\">");
    html.push_str(&escape_html(&display_name));
    html.push_str("</h1><p class=\"font-interface-md text-interface-md text-on-surface-variant max-w-[720px] mb-6\">");
    html.push_str(&author_bio(&profile.username));
    html.push_str("</p><div class=\"flex flex-wrap gap-4 mb-6\"><span class=\"rounded-full bg-surface-container px-4 py-2 text-on-surface-variant\">");
    html.push_str(&profile.article_count.to_string());
    html.push_str(" 篇文章</span><span class=\"rounded-full bg-surface-container px-4 py-2 text-on-surface-variant\">");
    html.push_str(&profile.follower_count.to_string());
    html.push_str(" 位关注者</span></div><button class=\"bg-primary text-on-primary px-6 py-3 rounded-lg font-interface-md text-interface-md font-bold\" type=\"button\" data-follow-author data-author-id=\"");
    html.push_str(&profile.id.to_string());
    html.push_str("\" data-author-name=\"");
    html.push_str(&escape_html(&display_name));
    html.push_str("\" data-following=\"false\">关注 ");
    html.push_str(&escape_html(&display_name));
    html.push_str("</button></div></div></section><div class=\"grid grid-cols-1 lg:grid-cols-12 gap-12\"><section class=\"lg:col-span-8 space-y-8\"><div class=\"flex items-center justify-between border-b border-outline-variant pb-4\"><h2 class=\"font-headline-md text-headline-md\">作者文章</h2><span class=\"text-on-surface-variant\">");
    html.push_str(&articles.len().to_string());
    html.push_str(" 篇文章</span></div>");
    render_article_grid(&mut html, articles);
    if articles.is_empty() {
        html.push_str("<div class=\"text-center py-20\"><span class=\"material-symbols-outlined text-[64px] text-on-surface-variant opacity-40\">article</span><p class=\"font-interface-md text-interface-md text-on-surface-variant mt-4\">该作者暂无已发布文章。</p></div>");
    }
    html.push_str("</section><aside class=\"lg:col-span-4 space-y-8\">");
    html.push_str(&sidebar_newsletter());
    html.push_str(&sidebar_categories(categories, None));
    html.push_str("</aside></div></main>");
    html.push_str(&site_footer(config));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn render_tag_page(
    config: &Config,
    slug: &str,
    label: &str,
    articles: &[PublicArticleSummary],
    categories: &[PublicCategoryWithCount],
) -> String {
    let mut html = document_start(
        "tag",
        &format!("{} &mdash; {}", label, config.site.title),
        &config.site.description,
        "bg-background text-on-surface",
        false,
    );
    html.push_str(&topnav(config, ""));
    html.push_str("<main class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop pt-10 pb-20\"><section class=\"mb-12 pb-8 border-b border-outline-variant\"><p class=\"text-primary font-label-sm text-label-sm uppercase tracking-wider mb-3\">标签文章</p><h1 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg leading-tight mb-4\">");
    html.push_str(&escape_html(label));
    html.push_str("</h1><p class=\"font-interface-md text-interface-md text-on-surface-variant max-w-[720px]\">围绕该标签整理的公开文章，适合按主题连续阅读。</p></section><div class=\"grid grid-cols-1 lg:grid-cols-12 gap-12\"><section class=\"lg:col-span-8 space-y-8\">");
    render_article_grid(&mut html, articles);
    if articles.is_empty() {
        html.push_str("<div class=\"text-center py-20\"><span class=\"material-symbols-outlined text-[64px] text-on-surface-variant opacity-40\">sell</span><p class=\"font-interface-md text-interface-md text-on-surface-variant mt-4\">该标签下暂无文章。</p></div>");
    }
    html.push_str("</section><aside class=\"lg:col-span-4 space-y-8\">");
    html.push_str(&sidebar_newsletter());
    html.push_str("<div class=\"p-6 rounded-xl bg-surface-container-low border border-outline-variant\"><h3 class=\"font-interface-md text-interface-md font-bold mb-6 flex items-center gap-2\"><span class=\"material-symbols-outlined text-primary\">sell</span> 标签云</h3><div class=\"flex flex-wrap gap-2\">");
    for (tag_slug, tag_name) in tag_cloud(categories) {
        html.push_str("<a class=\"rounded-full px-3 py-1 text-caption font-caption ");
        if tag_slug == slug {
            html.push_str("bg-primary text-on-primary");
        } else {
            html.push_str("bg-surface-container text-on-surface-variant hover:text-primary");
        }
        html.push_str("\" href=\"/tags/");
        html.push_str(&escape_html(&tag_slug));
        html.push_str("\">");
        html.push_str(&escape_html(&tag_name));
        html.push_str("</a>");
    }
    html.push_str("</div></div>");
    html.push_str(&sidebar_categories(categories, None));
    html.push_str("</aside></div></main>");
    html.push_str(&site_footer(config));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn render_archive_page(
    config: &Config,
    articles: &[PublicArticleSummary],
    categories: &[PublicCategoryWithCount],
) -> String {
    let mut html = document_start(
        "archive",
        &format!("文章归档 &mdash; {}", config.site.title),
        &config.site.description,
        "bg-background text-on-surface",
        false,
    );
    html.push_str(&topnav(config, ""));
    html.push_str("<main class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop pt-10 pb-20\"><section class=\"mb-12 pb-8 border-b border-outline-variant\"><p class=\"text-primary font-label-sm text-label-sm uppercase tracking-wider mb-3\">Archive</p><h1 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg leading-tight mb-4\">文章归档</h1><p class=\"font-interface-md text-interface-md text-on-surface-variant max-w-[720px]\">按发布时间回看所有已发布内容。</p></section><div class=\"grid grid-cols-1 lg:grid-cols-12 gap-12\"><section class=\"lg:col-span-8 space-y-10\">");
    let mut current_year = String::new();
    let mut current_month = String::new();
    for article in articles {
        let (year, month) = archive_parts(article.published_at.as_deref());
        if year != current_year {
            if !current_year.is_empty() {
                html.push_str("</div>");
            }
            current_year = year.clone();
            current_month.clear();
            html.push_str("<div class=\"space-y-6\"><h2 class=\"font-headline-md text-headline-md text-on-surface\">");
            html.push_str(&escape_html(&year));
            html.push_str(" 年</h2>");
        }
        if month != current_month {
            current_month = month.clone();
            html.push_str(
                "<h3 class=\"font-interface-md text-interface-md font-bold text-primary mt-6\">",
            );
            html.push_str(&escape_html(&month));
            html.push_str(" 月</h3>");
        }
        html.push_str("<article class=\"rounded-xl border border-outline-variant bg-surface-container-low p-5\"><a class=\"flex flex-col md:flex-row md:items-center gap-3 justify-between\" href=\"/articles/");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\" data-article-slug=\"");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\"><div><h4 class=\"font-interface-md text-interface-md font-bold hover:text-primary transition-colors\">");
        html.push_str(&escape_html(&article.title));
        html.push_str("</h4><p class=\"text-caption font-caption text-on-surface-variant mt-1\">");
        html.push_str(&escape_html(&article.excerpt));
        html.push_str(
            "</p></div><span class=\"text-caption font-caption text-on-surface-variant shrink-0\">",
        );
        html.push_str(&format_date(article.published_at.as_deref()));
        html.push_str("</span></a></article>");
    }
    if !current_year.is_empty() {
        html.push_str("</div>");
    }
    if articles.is_empty() {
        html.push_str("<div class=\"text-center py-20\"><span class=\"material-symbols-outlined text-[64px] text-on-surface-variant opacity-40\">archive</span><p class=\"font-interface-md text-interface-md text-on-surface-variant mt-4\">暂无归档文章。</p></div>");
    }
    html.push_str("</section><aside class=\"lg:col-span-4 space-y-8\">");
    html.push_str(&sidebar_newsletter());
    html.push_str(&sidebar_categories(categories, None));
    html.push_str("</aside></div></main>");
    html.push_str(&site_footer(config));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn render_not_found_page(config: &Config) -> String {
    let mut html = document_start(
        "not-found",
        &format!("页面未找到 &mdash; {}", config.site.title),
        &config.site.description,
        "bg-background text-on-surface",
        false,
    );
    html.push_str(&topnav(config, ""));
    html.push_str("<main class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop py-24\"><section class=\"max-w-[720px]\"><p class=\"text-primary font-label-sm text-label-sm uppercase tracking-wider mb-4\">404</p><h1 class=\"font-display-lg-mobile md:font-display-lg text-display-lg-mobile md:text-display-lg leading-tight mb-6\">页面未找到</h1><p class=\"font-interface-md text-interface-md text-on-surface-variant mb-10\">这个页面可能已经移动、删除，或链接地址输入有误。</p><div class=\"flex flex-wrap gap-4\"><a class=\"bg-primary text-on-primary px-6 py-3 rounded-lg font-interface-md text-interface-md font-bold\" href=\"/\">返回首页</a><a class=\"border border-outline-variant px-6 py-3 rounded-lg font-interface-md text-interface-md font-bold hover:bg-surface-container\" href=\"/categories\">浏览分类</a><a class=\"border border-outline-variant px-6 py-3 rounded-lg font-interface-md text-interface-md font-bold hover:bg-surface-container\" href=\"/archive\">查看归档</a></div></section></main>");
    html.push_str(&site_footer(config));
    html.push_str("<script src=\"/assets/site.js\" defer></script></body></html>");
    html
}

fn html_response(html: String) -> axum::response::Response {
    let mut response = html.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
}

fn document_start(
    page: &str,
    title: &str,
    description: &str,
    body_class: &str,
    include_article_style: bool,
) -> String {
    let mut html = String::from("<!doctype html><html lang=\"zh-CN\"><head><title>");
    html.push_str(&escape_html(title));
    html.push_str("</title><meta name=\"description\" content=\"");
    html.push_str(&escape_html(description));
    html.push_str("\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><link href=\"https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=Source+Serif+4:ital,opsz,wght@0,8..60,400;0,8..60,600;1,8..60,400&display=swap\" rel=\"stylesheet\"><link href=\"https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:wght,FILL@100..700,0..1&display=swap\" rel=\"stylesheet\"><link rel=\"stylesheet\" href=\"/assets/site.css\"><style>.material-symbols-outlined{font-variation-settings:'FILL' 0,'wght' 400,'GRAD' 0,'opsz' 24}body{font-family:'Inter',sans-serif;scroll-behavior:smooth}.reading-progress{position:fixed;top:0;left:0;height:4px;background-color:#0058be;z-index:100;width:0%;transition:width .1s linear}.article-html h1,.article-html h2,.article-html h3{font-family:'Inter',sans-serif;font-weight:600;color:#191c1d;margin-top:2.5rem;margin-bottom:1rem;line-height:1.3}.article-html h1{font-size:32px}.article-html h2{font-size:24px}.article-html h3{font-size:20px}.article-html p{margin-bottom:1.5rem}.article-html a{color:#0058be;text-decoration:underline}");
    if include_article_style {
        html.push_str(".article-html>p:first-child::first-letter{font-size:3.25rem;font-weight:700;line-height:1;margin-right:.75rem;float:left;color:#0058be}");
    }
    html.push_str("</style></head><body class=\"");
    html.push_str(body_class);
    html.push_str("\" data-page=\"");
    html.push_str(page);
    html.push_str("\">");
    html
}

fn topnav(config: &Config, keyword: &str) -> String {
    let mut html = String::from("<header class=\"bg-surface-container-lowest border-b border-outline-variant shadow-sm sticky top-0 z-40\"><div class=\"flex justify-between items-center w-full px-margin-mobile md:px-margin-desktop py-4 max-w-container-max mx-auto\"><a href=\"/\" class=\"font-display-lg font-bold text-on-surface text-2xl md:text-3xl tracking-tight\">");
    html.push_str(&escape_html(&config.site.title));
    html.push_str("</a><nav class=\"hidden md:flex items-center gap-8\"><a class=\"font-interface-md text-interface-md text-primary font-bold border-b-2 border-primary pb-1\" href=\"/\">最新</a><a class=\"font-interface-md text-interface-md text-on-surface-variant hover:text-primary transition-colors duration-200\" href=\"/categories\">分类</a><a class=\"font-interface-md text-interface-md text-on-surface-variant hover:text-primary transition-colors duration-200\" href=\"/about\">关于</a></nav><div class=\"flex items-center gap-4\"><button type=\"button\" class=\"hidden md:block text-on-surface-variant hover:text-primary transition-colors p-2\" aria-label=\"搜索\" data-search-toggle><span class=\"material-symbols-outlined\">search</span></button><a href=\"/admin\" class=\"font-interface-md text-interface-md text-on-surface-variant px-4 py-2 hover:text-primary transition-colors\">登录</a><button type=\"button\" class=\"bg-primary text-on-primary px-6 py-2 rounded-lg font-interface-md text-interface-md hover:bg-opacity-90 transition-all active:scale-95\" data-newsletter-focus>订阅</button></div></div><div class=\"hidden border-t border-outline-variant bg-surface-container-lowest\" data-search-panel><form action=\"/search\" class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop py-4 flex flex-col sm:flex-row gap-3\" data-search-form><label class=\"sr-only\" for=\"site-search\">搜索文章</label><input id=\"site-search\" name=\"keyword\" class=\"flex-1 rounded-lg border border-outline-variant bg-surface-container-lowest px-4 py-3 font-interface-md text-interface-md text-on-surface placeholder:text-on-surface-variant focus:border-primary focus:ring-2 focus:ring-primary/20\" placeholder=\"搜索文章标题或摘要\" value=\"");
    html.push_str(&escape_html(keyword));
    html.push_str("\"><button type=\"submit\" class=\"bg-primary text-on-primary px-6 py-3 rounded-lg font-interface-md text-interface-md font-bold\">搜索</button></form></div></header>");
    html
}

fn site_footer(config: &Config) -> String {
    let mut html = String::from("<footer class=\"bg-surface-container-lowest border-t border-outline-variant py-12 mt-20\"><div class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop flex flex-col md:flex-row justify-between items-center gap-8\"><div class=\"flex flex-col gap-2 items-center md:items-start\"><div class=\"font-headline-md text-headline-md font-bold text-on-surface\">");
    html.push_str(&escape_html(&config.site.title));
    html.push_str("</div><p class=\"font-caption text-caption text-on-surface-variant max-w-[320px] text-center md:text-left\">");
    html.push_str(&escape_html(&config.site.description));
    html.push_str("</p></div><div class=\"flex flex-col md:flex-row items-center gap-8\"><nav class=\"flex gap-6\"><a class=\"font-caption text-caption text-on-surface-variant hover:text-on-surface transition-colors\" href=\"/\">首页</a><a class=\"font-caption text-caption text-on-surface-variant hover:text-on-surface transition-colors\" href=\"/categories\">分类</a><a class=\"font-caption text-caption text-on-surface-variant hover:text-on-surface transition-colors\" href=\"/about\">关于</a><a class=\"font-caption text-caption text-on-surface-variant hover:text-on-surface transition-colors\" href=\"/admin\">后台</a></nav><div class=\"flex gap-4\"><button type=\"button\" class=\"w-10 h-10 rounded-full bg-surface-container flex items-center justify-center hover:bg-primary hover:text-on-primary transition-all\" aria-label=\"分享\" data-share-page><span class=\"material-symbols-outlined text-[20px]\">share</span></button><button type=\"button\" class=\"w-10 h-10 rounded-full bg-surface-container flex items-center justify-center hover:bg-primary hover:text-on-primary transition-all\" aria-label=\"邮件订阅\" data-newsletter-focus><span class=\"material-symbols-outlined text-[20px]\">mail</span></button></div></div></div><div class=\"max-w-container-max mx-auto px-margin-mobile md:px-margin-desktop mt-8 pt-8 border-t border-outline-variant text-center\"><p class=\"font-caption text-caption text-on-surface-variant\">&copy; ");
    html.push_str(&escape_html(&config.site.title));
    html.push_str("。保留所有权利。</p></div></footer>");
    html
}

fn sidebar_newsletter() -> String {
    "<div id=\"newsletter\" class=\"bg-primary-container p-6 rounded-xl text-on-primary-container shadow-sm border border-primary\"><h3 class=\"font-headline-md text-headline-md mb-3\">每周洞察</h3><p class=\"font-interface-md text-interface-md mb-5 opacity-90\">精选设计、技术与有意识生活方式文章，每周送达。</p><form class=\"space-y-3\" aria-label=\"订阅邮件\" data-newsletter-form><input class=\"w-full px-4 py-3 rounded-lg bg-surface-container-lowest text-on-surface border-none focus:ring-2 focus:ring-on-primary-container placeholder:text-outline-variant font-interface-md text-interface-md\" placeholder=\"email@address.com\" type=\"email\" name=\"email\"><button type=\"submit\" class=\"w-full bg-on-primary-container text-primary-container py-3 rounded-lg font-bold font-interface-md text-interface-md\">订阅提醒</button><p class=\"text-caption font-caption opacity-80\" data-newsletter-message role=\"status\"></p></form></div>".to_string()
}

fn sidebar_categories(categories: &[PublicCategoryWithCount], current: Option<&str>) -> String {
    let mut html = String::from("<div id=\"categories\" class=\"p-6 rounded-xl bg-surface-container-low border border-outline-variant\"><h3 class=\"font-interface-md text-interface-md font-bold mb-6 flex items-center gap-2\"><span class=\"material-symbols-outlined text-primary\">category</span> 分类</h3><ul class=\"space-y-3\"><li class=\"flex justify-between items-center group\"><a href=\"/\" class=\"font-interface-md text-interface-md text-on-surface-variant group-hover:text-primary transition-colors\">全部文章</a></li>");
    for category in categories {
        html.push_str("<li class=\"flex justify-between items-center group\" data-category-id=\"");
        html.push_str(&category.id.to_string());
        html.push_str("\"><a href=\"/categories/");
        html.push_str(&escape_html(&category.slug));
        html.push_str("\" class=\"font-interface-md text-interface-md group-hover:text-primary transition-colors ");
        if current == Some(category.slug.as_str()) {
            html.push_str("text-primary font-bold");
        } else {
            html.push_str("text-on-surface-variant");
        }
        html.push_str("\">");
        html.push_str(&escape_html(&category.name));
        html.push_str("</a><span class=\"text-caption font-caption bg-surface-container px-2 py-0.5 rounded-full text-on-surface-variant group-hover:bg-primary-container group-hover:text-on-primary-container transition-all\">");
        html.push_str(&category.article_count.to_string());
        html.push_str("</span></li>");
    }
    html.push_str("</ul></div>");
    html
}

fn render_article_grid(html: &mut String, articles: &[PublicArticleSummary]) {
    if articles.is_empty() {
        return;
    }
    html.push_str("<div class=\"grid grid-cols-1 md:grid-cols-2 gap-8\">");
    for article in articles {
        html.push_str("<article class=\"bg-surface-container-low rounded-lg border border-outline-variant overflow-hidden flex flex-col hover:shadow-lg transition-all hover:-translate-y-1\" data-article-slug=\"");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\"><a href=\"/articles/");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\" class=\"aspect-video overflow-hidden bg-surface-container-high block\">");
        render_article_image(html, article, "w-full h-full object-cover");
        html.push_str("</a><div class=\"p-6 flex-grow flex flex-col\"><div class=\"mb-3 flex items-center gap-2 flex-wrap\">");
        render_category_chip(html, article.category.as_ref());
        if article.is_pinned {
            html.push_str("<span class=\"bg-primary-container text-on-primary-container px-2 py-0.5 rounded text-caption font-label-sm\">精选</span>");
        }
        html.push_str("</div><h3 class=\"font-headline-md text-headline-md mb-3 leading-snug\"><a href=\"/articles/");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\" class=\"hover:text-primary transition-colors\">");
        html.push_str(&escape_html(&article.title));
        html.push_str("</a></h3><p class=\"font-interface-md text-interface-md text-on-surface-variant mb-6 line-clamp-2\">");
        html.push_str(&escape_html(&article.excerpt));
        html.push_str("</p><div class=\"mt-auto flex items-center justify-between\"><span class=\"text-caption font-caption text-on-surface-variant\">作者：");
        html.push_str(&author_name(&article.author.username));
        html.push_str(" &middot; ");
        html.push_str(&article.read_time_min.to_string());
        html.push_str(" 分钟阅读</span><button class=\"like-button flex items-center gap-1 text-on-surface-variant hover:text-error transition-colors\" type=\"button\" data-like-button data-slug=\"");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\" aria-label=\"喜欢文章\"><span class=\"material-symbols-outlined text-[20px] like-icon\">favorite_border</span><span class=\"like-count text-caption font-caption\">");
        html.push_str(&article.like_count.to_string());
        html.push_str("</span></button></div></div></article>");
    }
    html.push_str("</div>");
}

fn render_related_grid(html: &mut String, articles: &[PublicArticleSummary]) {
    html.push_str("<div class=\"grid grid-cols-1 md:grid-cols-3 gap-gutter\">");
    for article in articles {
        html.push_str("<a href=\"/articles/");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\" class=\"group cursor-pointer block\" data-article-slug=\"");
        html.push_str(&escape_html(&article.slug));
        html.push_str("\"><div class=\"aspect-[16/10] rounded-xl overflow-hidden mb-6 bg-surface-container-high transition-transform duration-300 group-hover:scale-[1.02]\">");
        render_article_image(html, article, "w-full h-full object-cover");
        html.push_str("</div>");
        if let Some(category) = &article.category {
            html.push_str("<span class=\"text-primary font-bold text-caption uppercase tracking-wider mb-2 block\">");
            html.push_str(&escape_html(&category.name));
            html.push_str("</span>");
        }
        html.push_str("<h4 class=\"font-headline-md text-headline-md mb-4 group-hover:text-primary transition-colors\">");
        html.push_str(&escape_html(&article.title));
        html.push_str("</h4><p class=\"text-on-surface-variant line-clamp-2\">");
        html.push_str(&escape_html(&article.excerpt));
        html.push_str("</p></a>");
    }
    html.push_str("</div>");
}

fn render_article_image(html: &mut String, article: &PublicArticleSummary, class_name: &str) {
    if article.cover_image.is_empty() {
        html.push_str("<div class=\"w-full h-full flex items-center justify-center text-on-surface-variant\"><span class=\"material-symbols-outlined text-[48px] opacity-40\">image</span></div>");
    } else {
        html.push_str("<img alt=\"");
        html.push_str(&escape_html(&article.title));
        html.push_str("\" class=\"");
        html.push_str(class_name);
        html.push_str("\" src=\"");
        html.push_str(&escape_html(&article.cover_image));
        html.push_str("\">");
    }
}

fn render_category_label(html: &mut String, category: Option<&PublicCategory>) {
    if let Some(category) = category {
        html.push_str(
            "<span class=\"text-primary font-label-sm text-label-sm uppercase tracking-wider\">",
        );
        html.push_str(&escape_html(&category.name));
        html.push_str(
            "</span><span class=\"text-outline text-label-sm font-label-sm\">&bull;</span>",
        );
    }
}

fn render_category_pill(html: &mut String, category: Option<&PublicCategory>) {
    if let Some(category) = category {
        html.push_str("<a href=\"/categories/");
        html.push_str(&escape_html(&category.slug));
        html.push_str("\" class=\"bg-primary-fixed text-on-primary-fixed px-3 py-1 rounded-full text-caption font-interface-md uppercase tracking-wider\">");
        html.push_str(&escape_html(&category.name));
        html.push_str("</a>");
    }
}

fn render_category_chip(html: &mut String, category: Option<&PublicCategory>) {
    if let Some(category) = category {
        html.push_str("<a href=\"/categories/");
        html.push_str(&escape_html(&category.slug));
        html.push_str("\" class=\"bg-tertiary-fixed text-on-tertiary-fixed px-2 py-0.5 rounded text-caption font-label-sm\">");
        html.push_str(&escape_html(&category.name));
        html.push_str("</a>");
    }
}

fn comment_form(slug: &str) -> String {
    let mut html = String::from("<div class=\"bg-surface-container-lowest p-6 rounded-xl shadow-sm border border-outline-variant mb-12\"><form data-comment-form data-slug=\"");
    html.push_str(&escape_html(slug));
    html.push_str("\" aria-label=\"发表评论\"><input type=\"hidden\" name=\"parent_id\" data-comment-parent-id><div class=\"hidden mb-4 rounded-lg bg-primary-container px-4 py-3 text-on-primary-container text-caption font-caption\" data-reply-target>正在回复 <strong data-reply-author></strong><button type=\"button\" class=\"ml-3 font-bold underline\" data-reply-cancel>取消</button></div><div class=\"grid grid-cols-1 md:grid-cols-[180px_1fr] gap-4\"><label class=\"sr-only\" for=\"comment-author\">昵称</label><input id=\"comment-author\" name=\"author_name\" class=\"w-full bg-surface-container-lowest border border-outline-variant rounded-lg focus:ring-2 focus:ring-primary font-interface-md text-interface-md text-on-surface placeholder-on-surface-variant\" placeholder=\"昵称\" maxlength=\"40\"><label class=\"sr-only\" for=\"comment-content\">评论内容</label><textarea id=\"comment-content\" name=\"content\" class=\"w-full bg-surface-container-lowest border border-outline-variant rounded-lg focus:ring-2 focus:ring-primary font-interface-md text-interface-md text-on-surface placeholder-on-surface-variant\" placeholder=\"加入讨论...\" rows=\"3\" maxlength=\"500\" required></textarea></div><p class=\"mt-3 text-caption text-on-surface-variant\">评论需保持友善、理性，不得包含政治、暴力、血腥等敏感内容。</p><p class=\"mt-2 text-caption text-on-surface-variant\" data-comment-message role=\"status\"></p><div class=\"flex justify-end mt-4\"><button type=\"submit\" class=\"bg-primary text-on-primary px-8 py-2 rounded-lg font-interface-md\">发表评论</button></div></form></div>");
    html
}

fn render_comments(html: &mut String, comments: &[PublicCommentNode]) {
    if comments.is_empty() {
        html.push_str("<div class=\"text-center py-8 text-on-surface-variant font-interface-md text-interface-md\">成为第一个分享想法的人。</div>");
        return;
    }
    html.push_str("<div class=\"space-y-8\">");
    for comment in comments {
        html.push_str("<div class=\"flex gap-4\" data-comment-id=\"");
        html.push_str(&comment.id.to_string());
        html.push_str(
            "\"><img class=\"w-10 h-10 rounded-full bg-outline-variant object-cover\" src=\"",
        );
        html.push_str(&author_avatar(&comment.author_name));
        html.push_str("\" alt=\"");
        html.push_str(&escape_html(&comment.author_name));
        html.push_str("\"><div class=\"flex-1\"><div class=\"flex items-center gap-3 mb-1\"><span class=\"font-bold text-on-surface\">");
        html.push_str(&escape_html(&comment.author_name));
        html.push_str("</span><span class=\"text-caption text-on-surface-variant\">刚刚</span></div><p class=\"text-on-surface-variant mb-3\">");
        html.push_str(&escape_html(&comment.content));
        html.push_str("</p><button class=\"text-primary text-caption font-bold flex items-center gap-1\" type=\"button\" data-comment-reply data-comment-id=\"");
        html.push_str(&comment.id.to_string());
        html.push_str("\" data-comment-author=\"");
        html.push_str(&escape_html(&comment.author_name));
        html.push_str(
            "\"><span class=\"material-symbols-outlined text-[16px]\">reply</span> 回复</button>",
        );
        if !comment.replies.is_empty() {
            html.push_str("<div class=\"mt-5 space-y-5 border-l border-outline-variant pl-5\">");
            for reply in &comment.replies {
                html.push_str("<div class=\"flex gap-3\" data-comment-id=\"");
                html.push_str(&reply.id.to_string());
                html.push_str(
                    "\"><img class=\"w-8 h-8 rounded-full bg-outline-variant object-cover\" src=\"",
                );
                html.push_str(&author_avatar(&reply.author_name));
                html.push_str("\" alt=\"");
                html.push_str(&escape_html(&reply.author_name));
                html.push_str("\"><div><div class=\"flex items-center gap-3 mb-1\"><span class=\"font-bold text-on-surface\">");
                html.push_str(&escape_html(&reply.author_name));
                html.push_str("</span><span class=\"text-caption text-on-surface-variant\">刚刚</span></div><p class=\"text-on-surface-variant\">");
                html.push_str(&escape_html(&reply.content));
                html.push_str("</p></div></div>");
            }
            html.push_str("</div>");
        }
        html.push_str("</div></div>");
    }
    html.push_str("</div>");
}

fn hidden_sidebar_fallback(categories: &[PublicCategoryWithCount]) -> String {
    let mut html = String::from("<div class=\"hidden\" aria-hidden=\"true\">");
    html.push_str(&sidebar_newsletter());
    html.push_str(&sidebar_categories(categories, None));
    html.push_str("</div>");
    html
}

fn format_date(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let date = value.split('T').next().unwrap_or(value);
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return escape_html(value);
    };
    format!(
        "{}年{}月{}日",
        year,
        month.trim_start_matches('0'),
        day.trim_start_matches('0')
    )
}

fn author_name(username: &str) -> String {
    match username {
        "admin" => "编辑部".into(),
        value if value.trim().is_empty() => "匿名作者".into(),
        value => escape_html(value),
    }
}

fn author_initial(username: &str) -> String {
    author_name(username)
        .chars()
        .next()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "A".into())
}

fn author_avatar(username: &str) -> String {
    let seed = if username.trim().is_empty() {
        "anonymous"
    } else {
        username.trim()
    };
    format!(
        "https://api.dicebear.com/7.x/initials/svg?seed={}",
        escape_url_component(seed)
    )
}

fn category_description(slug: &str, name: &str) -> String {
    match slug {
        "technology" => "技术趋势、工程实践和软件系统的深入观察。".into(),
        "design" => "产品体验、界面系统和信息表达的设计分析。".into(),
        "lifestyle" => "关于专注、工作方式和长期创作节奏的记录。".into(),
        "editorial" => "写作流程、编辑判断和内容系统方法论。".into(),
        _ => format!("收录于「{}」主题下的精选文章。", name),
    }
}

fn tag_label(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn tag_cloud(categories: &[PublicCategoryWithCount]) -> Vec<(String, String)> {
    let mut tags = vec![
        ("design-systems".to_string(), "Design Systems".to_string()),
        ("rust".to_string(), "Rust".to_string()),
        ("editorial".to_string(), "Editorial".to_string()),
        ("workflow".to_string(), "Workflow".to_string()),
    ];
    for category in categories {
        tags.push((category.slug.clone(), category.name.clone()));
    }
    tags
}

fn archive_parts(value: Option<&str>) -> (String, String) {
    let Some(value) = value else {
        return ("未归档".into(), "未知".into());
    };
    let date = value.split('T').next().unwrap_or(value);
    let mut parts = date.split('-');
    let year = parts.next().unwrap_or("未归档").to_string();
    let month = parts
        .next()
        .map(|value| value.trim_start_matches('0'))
        .filter(|value| !value.is_empty())
        .unwrap_or("未知")
        .to_string();
    (year, month)
}

fn author_bio(username: &str) -> String {
    if username == "admin" {
        "关注内容系统、产品工程与可持续创作流程。".into()
    } else {
        "分享设计、技术与生活方式观察。".into()
    }
}

fn escape_url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

async fn serve_file(root: PathBuf, path: String) -> Result<axum::response::Response> {
    let Some(path) = clean_relative_path(&path) else {
        return Err(AppError::HttpStatus(404, "not_found".into()));
    };
    let full_path = root.join(path);
    let data = fs::read(&full_path)
        .await
        .map_err(|_| AppError::HttpStatus(404, "not_found".into()))?;
    let mut response = data.into_response();
    if let Some(content_type) = content_type_for(&full_path) {
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    }
    Ok(response)
}

fn clean_relative_path(path: &str) -> Option<PathBuf> {
    let path = FsPath::new(path);
    if path.is_absolute() {
        return None;
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => result.push(value),
            _ => return None,
        }
    }
    if result.as_os_str().is_empty() {
        None
    } else {
        Some(result)
    }
}

fn content_type_for(path: &FsPath) -> Option<&'static str> {
    match path.extension().and_then(|value| value.to_str()) {
        Some("css") => Some("text/css; charset=utf-8"),
        Some("js") => Some("application/javascript; charset=utf-8"),
        Some("html") => Some("text/html; charset=utf-8"),
        Some("json") => Some("application/json; charset=utf-8"),
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        Some("txt") => Some("text/plain; charset=utf-8"),
        _ => None,
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
