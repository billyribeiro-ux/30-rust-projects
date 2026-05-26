use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBookmark {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBookmark {
    pub url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub q: Option<String>,
    pub tag: Option<String>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::patch(update).delete(delete))
}

async fn list(
    State(pool): State<SqlitePool>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<Bookmark>>> {
    let q_text = q.q.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let tag = q.tag.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

    // We use GROUP_CONCAT to inline each bookmark's tag list, then optionally
    // filter by a tag name (joining bookmark_tags + tags) and a text query
    // (LIKE on title/description/url). We branch on which filters are
    // present because sqlx's query! macro can't infer types for dynamic SQL.
    let rows = match (q_text, tag) {
        (None, None) => {
            let r = sqlx::query!(
                r#"
                SELECT
                    b.id, b.url, b.title, b.description, b.created_at, b.updated_at,
                    COALESCE(GROUP_CONCAT(t.name, ','), '') AS "tags!: String"
                FROM bookmarks b
                LEFT JOIN bookmark_tags bt ON bt.bookmark_id = b.id
                LEFT JOIN tags t ON t.id = bt.tag_id
                GROUP BY b.id
                ORDER BY b.updated_at DESC
                "#,
            )
            .fetch_all(&pool)
            .await?;
            r.into_iter()
                .map(|r| Bookmark {
                    id: r.id.expect("id is non-null primary key"),
                    url: r.url,
                    title: r.title,
                    description: r.description,
                    created_at: parse_ts(&r.created_at),
                    updated_at: parse_ts(&r.updated_at),
                    tags: split_tags(&r.tags),
                })
                .collect()
        }
        (Some(text), None) => {
            let like = format!("%{}%", text);
            let r = sqlx::query!(
                r#"
                SELECT
                    b.id, b.url, b.title, b.description, b.created_at, b.updated_at,
                    COALESCE(GROUP_CONCAT(t.name, ','), '') AS "tags!: String"
                FROM bookmarks b
                LEFT JOIN bookmark_tags bt ON bt.bookmark_id = b.id
                LEFT JOIN tags t ON t.id = bt.tag_id
                WHERE b.title LIKE ?1 OR b.description LIKE ?1 OR b.url LIKE ?1
                GROUP BY b.id
                ORDER BY b.updated_at DESC
                "#,
                like,
            )
            .fetch_all(&pool)
            .await?;
            r.into_iter()
                .map(|r| Bookmark {
                    id: r.id.expect("id is non-null primary key"),
                    url: r.url,
                    title: r.title,
                    description: r.description,
                    created_at: parse_ts(&r.created_at),
                    updated_at: parse_ts(&r.updated_at),
                    tags: split_tags(&r.tags),
                })
                .collect()
        }
        (None, Some(tag_name)) => {
            let r = sqlx::query!(
                r#"
                SELECT
                    b.id, b.url, b.title, b.description, b.created_at, b.updated_at,
                    COALESCE(GROUP_CONCAT(t.name, ','), '') AS "tags!: String"
                FROM bookmarks b
                JOIN bookmark_tags bt0 ON bt0.bookmark_id = b.id
                JOIN tags t0 ON t0.id = bt0.tag_id AND t0.name = ?1
                LEFT JOIN bookmark_tags bt ON bt.bookmark_id = b.id
                LEFT JOIN tags t ON t.id = bt.tag_id
                GROUP BY b.id
                ORDER BY b.updated_at DESC
                "#,
                tag_name,
            )
            .fetch_all(&pool)
            .await?;
            r.into_iter()
                .map(|r| Bookmark {
                    id: r.id.expect("id is non-null primary key"),
                    url: r.url,
                    title: r.title,
                    description: r.description,
                    created_at: parse_ts(&r.created_at),
                    updated_at: parse_ts(&r.updated_at),
                    tags: split_tags(&r.tags),
                })
                .collect()
        }
        (Some(text), Some(tag_name)) => {
            let like = format!("%{}%", text);
            let r = sqlx::query!(
                r#"
                SELECT
                    b.id, b.url, b.title, b.description, b.created_at, b.updated_at,
                    COALESCE(GROUP_CONCAT(t.name, ','), '') AS "tags!: String"
                FROM bookmarks b
                JOIN bookmark_tags bt0 ON bt0.bookmark_id = b.id
                JOIN tags t0 ON t0.id = bt0.tag_id AND t0.name = ?1
                LEFT JOIN bookmark_tags bt ON bt.bookmark_id = b.id
                LEFT JOIN tags t ON t.id = bt.tag_id
                WHERE b.title LIKE ?2 OR b.description LIKE ?2 OR b.url LIKE ?2
                GROUP BY b.id
                ORDER BY b.updated_at DESC
                "#,
                tag_name,
                like,
            )
            .fetch_all(&pool)
            .await?;
            r.into_iter()
                .map(|r| Bookmark {
                    id: r.id.expect("id is non-null primary key"),
                    url: r.url,
                    title: r.title,
                    description: r.description,
                    created_at: parse_ts(&r.created_at),
                    updated_at: parse_ts(&r.updated_at),
                    tags: split_tags(&r.tags),
                })
                .collect()
        }
    };

    Ok(Json(rows))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateBookmark>,
) -> AppResult<(StatusCode, Json<Bookmark>)> {
    let url = normalize_url(&payload.url)?;
    let title = normalize_title(&payload.title)?;
    let description = normalize_description(payload.description.as_deref().unwrap_or(""))?;
    let tag_names = normalize_tags(&payload.tags)?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = format_ts(now);

    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
        INSERT INTO bookmarks (id, url, title, description, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
        "#,
        id,
        url,
        title,
        description,
        now_str,
    )
    .execute(&mut *tx)
    .await?;

    set_bookmark_tags(&mut tx, &id, &tag_names).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(Bookmark {
            id,
            url,
            title,
            description,
            created_at: now,
            updated_at: now,
            tags: tag_names,
        }),
    ))
}

async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateBookmark>,
) -> AppResult<Json<Bookmark>> {
    let url = payload.url.as_deref().map(normalize_url).transpose()?;
    let title = payload.title.as_deref().map(normalize_title).transpose()?;
    let description = payload
        .description
        .as_deref()
        .map(normalize_description)
        .transpose()?;
    let tag_names = payload
        .tags
        .as_ref()
        .map(|t| normalize_tags(t))
        .transpose()?;

    let mut tx = pool.begin().await?;
    let now = Utc::now();
    let now_str = format_ts(now);

    let row = sqlx::query!(
        r#"
        UPDATE bookmarks
        SET url         = COALESCE(?1, url),
            title       = COALESCE(?2, title),
            description = COALESCE(?3, description),
            updated_at  = ?4
        WHERE id = ?5
        RETURNING id, url, title, description, created_at, updated_at
        "#,
        url,
        title,
        description,
        now_str,
        id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    let final_tags = if let Some(tags) = tag_names {
        set_bookmark_tags(&mut tx, &id, &tags).await?;
        tags
    } else {
        fetch_tags_for(&mut tx, &id).await?
    };

    tx.commit().await?;

    Ok(Json(Bookmark {
        id: row.id.expect("id is non-null primary key"),
        url: row.url,
        title: row.title,
        description: row.description,
        created_at: parse_ts(&row.created_at),
        updated_at: parse_ts(&row.updated_at),
        tags: final_tags,
    }))
}

async fn delete(State(pool): State<SqlitePool>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM bookmarks WHERE id = ?1", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------- helpers ----------------

async fn set_bookmark_tags(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    bookmark_id: &str,
    tag_names: &[String],
) -> AppResult<()> {
    sqlx::query!(
        "DELETE FROM bookmark_tags WHERE bookmark_id = ?1",
        bookmark_id
    )
    .execute(&mut **tx)
    .await?;

    for name in tag_names {
        let tag_id = upsert_tag(tx, name).await?;
        sqlx::query!(
            "INSERT OR IGNORE INTO bookmark_tags (bookmark_id, tag_id) VALUES (?1, ?2)",
            bookmark_id,
            tag_id,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn upsert_tag(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, name: &str) -> AppResult<String> {
    if let Some(row) = sqlx::query!("SELECT id FROM tags WHERE name = ?1", name)
        .fetch_optional(&mut **tx)
        .await?
    {
        return Ok(row.id.expect("id is non-null primary key"));
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query!("INSERT INTO tags (id, name) VALUES (?1, ?2)", id, name)
        .execute(&mut **tx)
        .await?;
    Ok(id)
}

async fn fetch_tags_for(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    bookmark_id: &str,
) -> AppResult<Vec<String>> {
    let rows = sqlx::query!(
        r#"
        SELECT t.name FROM tags t
        JOIN bookmark_tags bt ON bt.tag_id = t.id
        WHERE bt.bookmark_id = ?1
        ORDER BY t.name ASC
        "#,
        bookmark_id,
    )
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows.into_iter().map(|r| r.name).collect())
}

fn split_tags(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<String> = raw.split(',').map(|s| s.to_string()).collect();
    out.sort();
    out.dedup();
    out
}

fn normalize_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("url must not be empty".into()));
    }
    if trimmed.chars().count() > 2000 {
        return Err(AppError::Validation(
            "url must be 2000 chars or fewer".into(),
        ));
    }
    let parsed = url::Url::parse(trimmed)
        .map_err(|_| AppError::Validation("url must be a valid absolute URL".into()))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::Validation("url must be http or https".into()));
    }
    Ok(trimmed.to_string())
}

fn normalize_title(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("title must not be empty".into()));
    }
    if trimmed.chars().count() > 200 {
        return Err(AppError::Validation(
            "title must be 200 chars or fewer".into(),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_description(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > 1000 {
        return Err(AppError::Validation(
            "description must be 1000 chars or fewer".into(),
        ));
    }
    Ok(trimmed.to_string())
}

pub fn normalize_tags(raw: &[String]) -> AppResult<Vec<String>> {
    let mut out = Vec::with_capacity(raw.len());
    let mut seen = std::collections::HashSet::new();
    for t in raw {
        let trimmed = t.trim().to_lowercase();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.chars().count() > 40 {
            return Err(AppError::Validation("tag must be 40 chars or fewer".into()));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::Validation(
                "tag may only contain a-z, 0-9, hyphen, underscore".into(),
            ));
        }
        if seen.insert(trimmed.clone()) {
            out.push(trimmed);
        }
    }
    if out.len() > 20 {
        return Err(AppError::Validation(
            "a bookmark may have at most 20 tags".into(),
        ));
    }
    Ok(out)
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_accepts_https() {
        assert_eq!(
            normalize_url("https://example.com/path").unwrap(),
            "https://example.com/path"
        );
    }

    #[test]
    fn normalize_url_rejects_ftp() {
        assert!(matches!(
            normalize_url("ftp://example.com"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_url_rejects_relative() {
        assert!(matches!(
            normalize_url("/admin"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_url_rejects_empty() {
        assert!(matches!(normalize_url("   "), Err(AppError::Validation(_))));
    }

    #[test]
    fn normalize_tags_dedups_and_lowercases() {
        let r = normalize_tags(&["Rust".into(), "rust".into(), "axum".into()]).unwrap();
        assert_eq!(r, vec!["rust".to_string(), "axum".to_string()]);
    }

    #[test]
    fn normalize_tags_rejects_special_chars() {
        assert!(matches!(
            normalize_tags(&["bad tag!".into()]),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_tags_skips_empties() {
        let r = normalize_tags(&["".into(), "  ".into(), "rust".into()]).unwrap();
        assert_eq!(r, vec!["rust".to_string()]);
    }

    #[test]
    fn split_tags_handles_empty() {
        assert!(split_tags("").is_empty());
    }

    #[test]
    fn split_tags_sorts_and_dedups() {
        assert_eq!(split_tags("rust,axum,rust"), vec!["axum", "rust"]);
    }
}
