//! Contacts CRUD with:
//! - Per-user scoping (every query filters by user_id from the AuthUser
//!   extractor — never trust client-supplied user_id)
//! - Soft-delete via `deleted_at` (DELETE → set timestamp; restore endpoint
//!   exists; permanent purge is admin-only and not exposed yet)
//! - tsvector full-text search via the generated `search_tsv` column +
//!   `plainto_tsquery` for forgiving user input
//! - Tag management via the contact_tags junction (replace-all semantics on
//!   PATCH)

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct Contact {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub company: String,
    pub notes: String,
    pub last_contacted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateContact {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContact {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub touch_last_contacted: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub q: Option<String>,
    pub tag: Option<String>,
    pub include_deleted: Option<bool>,
    pub limit: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(read).patch(update).delete(soft_delete))
        .route("/{id}/restore", post(restore))
        .route("/{id}/touch", post(touch))
}

async fn list(
    State(s): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<Contact>>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let include_deleted = q.include_deleted.unwrap_or(false);
    let q_text = q.q.as_deref().map(|t| t.trim()).filter(|t| !t.is_empty());
    let tag = q.tag.as_deref().map(|t| t.trim()).filter(|t| !t.is_empty());

    // We branch on the (q, tag) combination because sqlx::query! can't infer
    // dynamic SQL. Same pattern as project 05.
    let rows = match (q_text, tag) {
        (None, None) => sqlx::query!(
            r#"
            SELECT c.id, c.name, c.email, c.phone, c.company, c.notes,
                   c.last_contacted_at, c.created_at, c.updated_at, c.deleted_at,
                   COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}')
                       AS "tags!: Vec<String>"
            FROM contacts c
            LEFT JOIN contact_tags ct ON ct.contact_id = c.id
            LEFT JOIN tags t ON t.id = ct.tag_id
            WHERE c.user_id = $1 AND ($2 OR c.deleted_at IS NULL)
            GROUP BY c.id
            ORDER BY c.updated_at DESC
            LIMIT $3
            "#,
            user.id,
            include_deleted,
            limit,
        )
        .fetch_all(&s.pool)
        .await?
        .into_iter()
        .map(|r| Contact {
            id: r.id,
            name: r.name,
            email: r.email,
            phone: r.phone,
            company: r.company,
            notes: r.notes,
            last_contacted_at: r.last_contacted_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            deleted_at: r.deleted_at,
            tags: r.tags,
        })
        .collect::<Vec<_>>(),

        (Some(query), None) => sqlx::query!(
            r#"
            SELECT c.id, c.name, c.email, c.phone, c.company, c.notes,
                   c.last_contacted_at, c.created_at, c.updated_at, c.deleted_at,
                   COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}')
                       AS "tags!: Vec<String>"
            FROM contacts c
            LEFT JOIN contact_tags ct ON ct.contact_id = c.id
            LEFT JOIN tags t ON t.id = ct.tag_id
            WHERE c.user_id = $1
              AND ($2 OR c.deleted_at IS NULL)
              AND c.search_tsv @@ plainto_tsquery('simple', $3)
            GROUP BY c.id
            ORDER BY ts_rank(c.search_tsv, plainto_tsquery('simple', $3)) DESC,
                     c.updated_at DESC
            LIMIT $4
            "#,
            user.id,
            include_deleted,
            query,
            limit,
        )
        .fetch_all(&s.pool)
        .await?
        .into_iter()
        .map(|r| Contact {
            id: r.id,
            name: r.name,
            email: r.email,
            phone: r.phone,
            company: r.company,
            notes: r.notes,
            last_contacted_at: r.last_contacted_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            deleted_at: r.deleted_at,
            tags: r.tags,
        })
        .collect::<Vec<_>>(),

        (None, Some(tag_name)) => sqlx::query!(
            r#"
            SELECT c.id, c.name, c.email, c.phone, c.company, c.notes,
                   c.last_contacted_at, c.created_at, c.updated_at, c.deleted_at,
                   COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}')
                       AS "tags!: Vec<String>"
            FROM contacts c
            JOIN contact_tags ct0 ON ct0.contact_id = c.id
            JOIN tags t0 ON t0.id = ct0.tag_id AND t0.user_id = c.user_id AND t0.name = $3
            LEFT JOIN contact_tags ct ON ct.contact_id = c.id
            LEFT JOIN tags t ON t.id = ct.tag_id
            WHERE c.user_id = $1 AND ($2 OR c.deleted_at IS NULL)
            GROUP BY c.id
            ORDER BY c.updated_at DESC
            LIMIT $4
            "#,
            user.id,
            include_deleted,
            tag_name,
            limit,
        )
        .fetch_all(&s.pool)
        .await?
        .into_iter()
        .map(|r| Contact {
            id: r.id,
            name: r.name,
            email: r.email,
            phone: r.phone,
            company: r.company,
            notes: r.notes,
            last_contacted_at: r.last_contacted_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            deleted_at: r.deleted_at,
            tags: r.tags,
        })
        .collect::<Vec<_>>(),

        (Some(query), Some(tag_name)) => sqlx::query!(
            r#"
            SELECT c.id, c.name, c.email, c.phone, c.company, c.notes,
                   c.last_contacted_at, c.created_at, c.updated_at, c.deleted_at,
                   COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}')
                       AS "tags!: Vec<String>"
            FROM contacts c
            JOIN contact_tags ct0 ON ct0.contact_id = c.id
            JOIN tags t0 ON t0.id = ct0.tag_id AND t0.user_id = c.user_id AND t0.name = $3
            LEFT JOIN contact_tags ct ON ct.contact_id = c.id
            LEFT JOIN tags t ON t.id = ct.tag_id
            WHERE c.user_id = $1
              AND ($2 OR c.deleted_at IS NULL)
              AND c.search_tsv @@ plainto_tsquery('simple', $4)
            GROUP BY c.id
            ORDER BY ts_rank(c.search_tsv, plainto_tsquery('simple', $4)) DESC,
                     c.updated_at DESC
            LIMIT $5
            "#,
            user.id,
            include_deleted,
            tag_name,
            query,
            limit,
        )
        .fetch_all(&s.pool)
        .await?
        .into_iter()
        .map(|r| Contact {
            id: r.id,
            name: r.name,
            email: r.email,
            phone: r.phone,
            company: r.company,
            notes: r.notes,
            last_contacted_at: r.last_contacted_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            deleted_at: r.deleted_at,
            tags: r.tags,
        })
        .collect::<Vec<_>>(),
    };

    Ok(Json(rows))
}

async fn read(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Contact>> {
    let row = sqlx::query!(
        r#"
        SELECT c.id, c.name, c.email, c.phone, c.company, c.notes,
               c.last_contacted_at, c.created_at, c.updated_at, c.deleted_at,
               COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}')
                   AS "tags!: Vec<String>"
        FROM contacts c
        LEFT JOIN contact_tags ct ON ct.contact_id = c.id
        LEFT JOIN tags t ON t.id = ct.tag_id
        WHERE c.id = $1 AND c.user_id = $2
        GROUP BY c.id
        "#,
        id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(Contact {
        id: row.id,
        name: row.name,
        email: row.email,
        phone: row.phone,
        company: row.company,
        notes: row.notes,
        last_contacted_at: row.last_contacted_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
        tags: row.tags,
    }))
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateContact>,
) -> AppResult<(StatusCode, Json<Contact>)> {
    let name = normalize_name(&input.name)?;
    let email = input.email.unwrap_or_default().trim().to_string();
    let phone = input.phone.unwrap_or_default().trim().to_string();
    let company = input.company.unwrap_or_default().trim().to_string();
    let notes = input.notes.unwrap_or_default().trim().to_string();
    let tag_names = normalize_tags(input.tags.as_deref().unwrap_or(&[]))?;

    let id = Uuid::new_v4();
    let mut tx = s.pool.begin().await?;
    sqlx::query!(
        r#"
        INSERT INTO contacts (id, user_id, name, email, phone, company, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        id,
        user.id,
        name,
        email,
        phone,
        company,
        notes,
    )
    .execute(&mut *tx)
    .await?;

    apply_tags(&mut tx, user.id, id, &tag_names).await?;
    tx.commit().await?;

    let now = Utc::now();
    Ok((
        StatusCode::CREATED,
        Json(Contact {
            id,
            name,
            email,
            phone,
            company,
            notes,
            last_contacted_at: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            tags: tag_names,
        }),
    ))
}

async fn update(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateContact>,
) -> AppResult<Json<Contact>> {
    let name = input.name.as_deref().map(normalize_name).transpose()?;
    let touch = input.touch_last_contacted.unwrap_or(false);

    let mut tx = s.pool.begin().await?;
    let result = sqlx::query!(
        r#"
        UPDATE contacts
        SET name    = COALESCE($1, name),
            email   = COALESCE($2, email),
            phone   = COALESCE($3, phone),
            company = COALESCE($4, company),
            notes   = COALESCE($5, notes),
            last_contacted_at = CASE WHEN $6 THEN now() ELSE last_contacted_at END
        WHERE id = $7 AND user_id = $8
        "#,
        name,
        input.email.as_ref().map(|s| s.trim()),
        input.phone.as_ref().map(|s| s.trim()),
        input.company.as_ref().map(|s| s.trim()),
        input.notes.as_ref().map(|s| s.trim()),
        touch,
        id,
        user.id,
    )
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    if let Some(tags) = input.tags {
        let normalized = normalize_tags(&tags)?;
        sqlx::query!("DELETE FROM contact_tags WHERE contact_id = $1", id)
            .execute(&mut *tx)
            .await?;
        apply_tags(&mut tx, user.id, id, &normalized).await?;
    }
    tx.commit().await?;

    // Re-fetch for canonical shape
    drop_and_read(&s, &user, id).await
}

async fn drop_and_read(s: &AppState, user: &AuthUser, id: Uuid) -> AppResult<Json<Contact>> {
    let row = sqlx::query!(
        r#"
        SELECT c.id, c.name, c.email, c.phone, c.company, c.notes,
               c.last_contacted_at, c.created_at, c.updated_at, c.deleted_at,
               COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}')
                   AS "tags!: Vec<String>"
        FROM contacts c
        LEFT JOIN contact_tags ct ON ct.contact_id = c.id
        LEFT JOIN tags t ON t.id = ct.tag_id
        WHERE c.id = $1 AND c.user_id = $2
        GROUP BY c.id
        "#,
        id,
        user.id,
    )
    .fetch_one(&s.pool)
    .await?;

    Ok(Json(Contact {
        id: row.id,
        name: row.name,
        email: row.email,
        phone: row.phone,
        company: row.company,
        notes: row.notes,
        last_contacted_at: row.last_contacted_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
        tags: row.tags,
    }))
}

async fn soft_delete(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!(
        "UPDATE contacts SET deleted_at = now() WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
        id,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn restore(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!(
        "UPDATE contacts SET deleted_at = NULL WHERE id = $1 AND user_id = $2 AND deleted_at IS NOT NULL",
        id,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn touch(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!(
        "UPDATE contacts SET last_contacted_at = now() WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
        id,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---- tag helpers ----

async fn apply_tags(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    contact_id: Uuid,
    tag_names: &[String],
) -> AppResult<()> {
    for name in tag_names {
        let tag_id = upsert_tag(tx, user_id, name).await?;
        sqlx::query!(
            "INSERT INTO contact_tags (contact_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            contact_id,
            tag_id,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn upsert_tag(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    name: &str,
) -> AppResult<Uuid> {
    // ON CONFLICT (user_id, name) DO UPDATE … RETURNING id — atomic upsert.
    let row = sqlx::query!(
        r#"
        INSERT INTO tags (id, user_id, name)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, name) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
        Uuid::new_v4(),
        user_id,
        name,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.id)
}

fn normalize_name(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "required".into(),
        }]));
    }
    if trimmed.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "must be 200 chars or fewer".into(),
        }]));
    }
    Ok(trimmed.to_string())
}

fn normalize_tags(raw: &[String]) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for t in raw {
        let trimmed = t.trim().to_lowercase();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.chars().count() > 40 {
            return Err(AppError::Fields(vec![FieldError {
                field: "tags".into(),
                message: "each tag must be 40 chars or fewer".into(),
            }]));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::Fields(vec![FieldError {
                field: "tags".into(),
                message: "tags may only contain a-z, 0-9, hyphen, underscore".into(),
            }]));
        }
        if seen.insert(trimmed.clone()) {
            out.push(trimmed);
        }
    }
    Ok(out)
}
