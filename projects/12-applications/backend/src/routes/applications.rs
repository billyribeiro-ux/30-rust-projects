//! Applications CRUD. Per-user scoped (server-enforced), with a status
//! whitelist matched by the DB CHECK constraint.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

const STATUSES: &[&str] = &[
    "wishlist",
    "applied",
    "screening",
    "interview",
    "offer",
    "accepted",
    "rejected",
    "withdrawn",
];

#[derive(Debug, Serialize)]
pub struct Application {
    pub id: Uuid,
    pub company: String,
    pub role: String,
    pub location: String,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub job_url: String,
    pub notes: String,
    pub status: String,
    pub applied_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApplication {
    pub company: String,
    pub role: String,
    pub location: Option<String>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub job_url: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplication {
    pub company: Option<String>,
    pub role: Option<String>,
    pub location: Option<String>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub job_url: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(read).patch(update).delete(delete))
}

async fn list(
    State(s): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<Application>>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    let status_filter = q
        .status
        .as_deref()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty());

    let rows = match status_filter {
        Some(st) => sqlx::query!(
            r#"
            SELECT id, company, role, location, salary_min, salary_max, job_url, notes,
                   status, applied_at, created_at, updated_at
            FROM applications
            WHERE user_id = $1 AND status = $2
            ORDER BY updated_at DESC
            LIMIT $3
            "#,
            user.id,
            st,
            limit,
        )
        .fetch_all(&s.pool)
        .await?
        .into_iter()
        .map(|r| Application {
            id: r.id,
            company: r.company,
            role: r.role,
            location: r.location,
            salary_min: r.salary_min,
            salary_max: r.salary_max,
            job_url: r.job_url,
            notes: r.notes,
            status: r.status,
            applied_at: r.applied_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect::<Vec<_>>(),
        None => sqlx::query!(
            r#"
            SELECT id, company, role, location, salary_min, salary_max, job_url, notes,
                   status, applied_at, created_at, updated_at
            FROM applications
            WHERE user_id = $1
            ORDER BY updated_at DESC
            LIMIT $2
            "#,
            user.id,
            limit,
        )
        .fetch_all(&s.pool)
        .await?
        .into_iter()
        .map(|r| Application {
            id: r.id,
            company: r.company,
            role: r.role,
            location: r.location,
            salary_min: r.salary_min,
            salary_max: r.salary_max,
            job_url: r.job_url,
            notes: r.notes,
            status: r.status,
            applied_at: r.applied_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect::<Vec<_>>(),
    };

    Ok(Json(rows))
}

async fn read(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Application>> {
    let row = sqlx::query!(
        r#"
        SELECT id, company, role, location, salary_min, salary_max, job_url, notes,
               status, applied_at, created_at, updated_at
        FROM applications WHERE id = $1 AND user_id = $2
        "#,
        id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(Application {
        id: row.id,
        company: row.company,
        role: row.role,
        location: row.location,
        salary_min: row.salary_min,
        salary_max: row.salary_max,
        job_url: row.job_url,
        notes: row.notes,
        status: row.status,
        applied_at: row.applied_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateApplication>,
) -> AppResult<(StatusCode, Json<Application>)> {
    let mut errors = Vec::new();
    let company = match normalize_required(&input.company, "company", 200) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            String::new()
        }
    };
    let role = match normalize_required(&input.role, "role", 200) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            String::new()
        }
    };
    let status =
        normalize_status(input.status.as_deref().unwrap_or("applied")).unwrap_or_else(|e| {
            errors.push(e);
            String::new()
        });
    if !errors.is_empty() {
        return Err(AppError::Fields(errors));
    }

    let location = input.location.unwrap_or_default().trim().to_string();
    let job_url = input.job_url.unwrap_or_default().trim().to_string();
    let notes = input.notes.unwrap_or_default().trim().to_string();
    let id = Uuid::new_v4();

    // If the user creates an application already at applied+, stamp applied_at.
    let applied_at = if matches!(
        status.as_str(),
        "applied" | "screening" | "interview" | "offer" | "accepted" | "rejected" | "withdrawn"
    ) {
        Some(Utc::now())
    } else {
        None
    };

    let mut tx = s.pool.begin().await?;
    sqlx::query!(
        r#"
        INSERT INTO applications (id, user_id, company, role, location, salary_min,
                                  salary_max, job_url, notes, status, applied_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
        id,
        user.id,
        company,
        role,
        location,
        input.salary_min,
        input.salary_max,
        job_url,
        notes,
        status,
        applied_at,
    )
    .execute(&mut *tx)
    .await?;

    // Record an initial event so the timeline starts with the creation.
    sqlx::query!(
        r#"
        INSERT INTO application_events (id, application_id, user_id, kind, new_status, body)
        VALUES ($1, $2, $3, 'status_change', $4, $5)
        "#,
        Uuid::new_v4(),
        id,
        user.id,
        status,
        format!("Applied to {} for {}", company, role),
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let now = Utc::now();
    Ok((
        StatusCode::CREATED,
        Json(Application {
            id,
            company,
            role,
            location,
            salary_min: input.salary_min,
            salary_max: input.salary_max,
            job_url,
            notes,
            status,
            applied_at,
            created_at: now,
            updated_at: now,
        }),
    ))
}

async fn update(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateApplication>,
) -> AppResult<Json<Application>> {
    // FieldError → AppError::Fields wrap (FieldError doesn't directly impl From<AppError>).
    let company = input
        .company
        .as_deref()
        .map(|v| normalize_required(v, "company", 200))
        .transpose()
        .map_err(|e| AppError::Fields(vec![e]))?;
    let role = input
        .role
        .as_deref()
        .map(|v| normalize_required(v, "role", 200))
        .transpose()
        .map_err(|e| AppError::Fields(vec![e]))?;
    let new_status = input
        .status
        .as_deref()
        .map(normalize_status)
        .transpose()
        .map_err(|e| AppError::Fields(vec![e]))?;

    let mut tx = s.pool.begin().await?;

    // Fetch prior status BEFORE updating so we can emit a status_change event.
    let prior = sqlx::query!(
        "SELECT status FROM applications WHERE id = $1 AND user_id = $2 FOR UPDATE",
        id,
        user.id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    let result = sqlx::query!(
        r#"
        UPDATE applications
        SET company    = COALESCE($1, company),
            role       = COALESCE($2, role),
            location   = COALESCE($3, location),
            salary_min = COALESCE($4, salary_min),
            salary_max = COALESCE($5, salary_max),
            job_url    = COALESCE($6, job_url),
            notes      = COALESCE($7, notes),
            status     = COALESCE($8, status)
        WHERE id = $9 AND user_id = $10
        RETURNING id, company, role, location, salary_min, salary_max, job_url,
                  notes, status, applied_at, created_at, updated_at
        "#,
        company,
        role,
        input.location.as_ref().map(|s| s.trim()),
        input.salary_min,
        input.salary_max,
        input.job_url.as_ref().map(|s| s.trim()),
        input.notes.as_ref().map(|s| s.trim()),
        new_status.as_deref(),
        id,
        user.id,
    )
    .fetch_one(&mut *tx)
    .await?;

    // If status changed, record an event.
    if let Some(ns) = new_status.as_deref()
        && ns != prior.status
    {
        sqlx::query!(
            r#"
            INSERT INTO application_events (id, application_id, user_id, kind, new_status, body)
            VALUES ($1, $2, $3, 'status_change', $4, $5)
            "#,
            Uuid::new_v4(),
            id,
            user.id,
            ns,
            format!("{} → {}", prior.status, ns),
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(Application {
        id: result.id,
        company: result.company,
        role: result.role,
        location: result.location,
        salary_min: result.salary_min,
        salary_max: result.salary_max,
        job_url: result.job_url,
        notes: result.notes,
        status: result.status,
        applied_at: result.applied_at,
        created_at: result.created_at,
        updated_at: result.updated_at,
    }))
}

async fn delete(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!(
        "DELETE FROM applications WHERE id = $1 AND user_id = $2",
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

fn normalize_required(raw: &str, field: &str, max: usize) -> Result<String, FieldError> {
    let t = raw.trim();
    if t.is_empty() {
        return Err(FieldError {
            field: field.into(),
            message: "required".into(),
        });
    }
    if t.chars().count() > max {
        return Err(FieldError {
            field: field.into(),
            message: format!("must be {max} chars or fewer"),
        });
    }
    Ok(t.to_string())
}

fn normalize_status(raw: &str) -> Result<String, FieldError> {
    let t = raw.trim().to_lowercase();
    if !STATUSES.contains(&t.as_str()) {
        return Err(FieldError {
            field: "status".into(),
            message: format!("must be one of: {}", STATUSES.join(", ")),
        });
    }
    Ok(t)
}
