//! Event CRUD + range expansion.
//!
//! `GET /api/events?from=ISO&to=ISO&calendar_id=...` returns expanded
//! occurrences for the window. Recurring events are unrolled by the
//! `rrule` crate at request time (cheaper than materialising); the
//! unrolled occurrences carry the parent `event_id` so the UI knows to
//! "edit this/all" when modifying.
//!
//! `POST /api/events` creates a new event (one-off or RRULE).
//! `PATCH /api/events/{id}` updates the master row.
//! `DELETE /api/events/{id}` removes it.
//!
//! Permission discipline: every query joins the
//! `accessible_calendars(user_id)` view (owner + edit-shares for writes,
//! owner + any share for reads).

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, response::IntoResponse};
use chrono::{DateTime, Utc};
use rrule::{RRuleSet, Tz as RTz};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(range).post(create))
        .route("/{id}", axum::routing::patch(update).delete(delete))
}

#[derive(Serialize)]
pub struct EventRow {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub title: String,
    pub description: String,
    pub location: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub tz: String,
    pub rrule: Option<String>,
    pub all_day: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Occurrence {
    pub event_id: Uuid,
    pub calendar_id: Uuid,
    pub title: String,
    pub description: String,
    pub location: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub tz: String,
    pub all_day: bool,
    pub is_recurring: bool,
}

#[derive(Deserialize)]
pub struct RangeQuery {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub calendar_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct CreateInput {
    pub calendar_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub location: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    #[serde(default = "default_tz")]
    pub tz: String,
    #[serde(default)]
    pub rrule: Option<String>,
    #[serde(default)]
    pub all_day: bool,
}

#[derive(Deserialize)]
pub struct UpdateInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub tz: Option<String>,
    pub rrule: Option<Option<String>>, // Some(None) clears, None leaves alone
    pub all_day: Option<bool>,
}

fn default_tz() -> String {
    "UTC".to_string()
}

async fn range(
    State(s): State<AppState>,
    user: AuthUser,
    Query(q): Query<RangeQuery>,
) -> AppResult<Json<Vec<Occurrence>>> {
    if q.to <= q.from {
        return Err(AppError::Validation("to must be > from".into()));
    }
    // 90-day cap — keep the range bounded so a malicious "from=1900&to=2100"
    // doesn't burn the worker on RRULE expansion.
    if (q.to - q.from).num_days() > 90 {
        return Err(AppError::Validation("range may not exceed 90 days".into()));
    }

    // Pull every potentially-overlapping event for accessible calendars.
    // For recurring events, we deliberately load even rows whose first
    // occurrence is in the past — the RRULE expander will produce
    // in-window occurrences from them.
    let rows = sqlx::query_as!(
        EventRow,
        r#"
        SELECT e.id, e.calendar_id, e.title, e.description, e.location,
               e.start_at, e.end_at, e.tz, e.rrule, e.all_day,
               e.created_at, e.updated_at
        FROM events e
        JOIN calendars c ON c.id = e.calendar_id
        LEFT JOIN calendar_shares cs ON cs.calendar_id = c.id AND cs.user_id = $1
        WHERE (c.owner_id = $1 OR cs.user_id = $1)
          AND ($2::uuid IS NULL OR e.calendar_id = $2)
          AND (
            -- one-offs: must overlap the window
            (e.rrule IS NULL AND e.start_at < $4 AND e.end_at > $3)
            -- recurring: keep, expand on the server
            OR (e.rrule IS NOT NULL AND e.start_at < $4)
          )
        "#,
        user.id,
        q.calendar_id,
        q.from,
        q.to,
    )
    .fetch_all(&s.pool)
    .await?;

    let mut out: Vec<Occurrence> = Vec::new();
    for r in rows {
        if let Some(ref rrule_str) = r.rrule {
            // Expand RRULE inside the window.
            let occurrences = expand_rrule(&r, rrule_str, q.from, q.to)
                .map_err(|e| {
                    tracing::warn!(rrule = %rrule_str, err = %e, "skipping invalid RRULE");
                    e
                })
                .unwrap_or_default();
            for (start, end) in occurrences {
                out.push(Occurrence {
                    event_id: r.id,
                    calendar_id: r.calendar_id,
                    title: r.title.clone(),
                    description: r.description.clone(),
                    location: r.location.clone(),
                    start_at: start,
                    end_at: end,
                    tz: r.tz.clone(),
                    all_day: r.all_day,
                    is_recurring: true,
                });
            }
        } else {
            out.push(Occurrence {
                event_id: r.id,
                calendar_id: r.calendar_id,
                title: r.title,
                description: r.description,
                location: r.location,
                start_at: r.start_at,
                end_at: r.end_at,
                tz: r.tz,
                all_day: r.all_day,
                is_recurring: false,
            });
        }
    }

    out.sort_by_key(|o| o.start_at);
    Ok(Json(out))
}

/// Expand `rrule_str` to all (start, end) pairs in [from, to).
/// Capped at 366 occurrences to bound work.
fn expand_rrule(
    e: &EventRow,
    rrule_str: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
    let tz: RTz =
        e.tz.parse::<chrono_tz::Tz>()
            .map_err(|_| AppError::Validation("invalid tz on event".into()))?
            .into();
    let dtstart = e.start_at.with_timezone(&tz);
    let duration = e.end_at - e.start_at;

    // The rrule crate expects DTSTART + RRULE as a combined string.
    let combined = format!(
        "DTSTART;TZID={}:{}\n{}",
        e.tz,
        dtstart.format("%Y%m%dT%H%M%S"),
        rrule_str.trim()
    );
    let set: RRuleSet = RRuleSet::from_str(&combined)
        .map_err(|err| AppError::Validation(format!("invalid RRULE: {err}")))?
        .after(from.with_timezone(&tz))
        .before(to.with_timezone(&tz));

    let result = set.all(366);
    let mut out = Vec::with_capacity(result.dates.len());
    for start in result.dates {
        let start_utc = start.with_timezone(&Utc);
        out.push((start_utc, start_utc + duration));
    }
    Ok(out)
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    let title = input.title.trim().to_string();
    if title.is_empty() || title.len() > 200 {
        return Err(AppError::Validation("title must be 1..=200 chars".into()));
    }
    if input.end_at < input.start_at {
        return Err(AppError::Validation("end_at must be >= start_at".into()));
    }
    if input.tz.parse::<chrono_tz::Tz>().is_err() {
        return Err(AppError::Validation("tz must be a valid IANA zone".into()));
    }

    // Caller must have write access (owner or edit-share) to the calendar.
    let can_write = sqlx::query_scalar!(
        r#"
        SELECT 1 AS x FROM calendars c
        LEFT JOIN calendar_shares cs ON cs.calendar_id = c.id AND cs.user_id = $1
        WHERE c.id = $2 AND (c.owner_id = $1 OR cs.permission = 'edit')
        "#,
        user.id,
        input.calendar_id,
    )
    .fetch_optional(&s.pool)
    .await?;
    if can_write.is_none() {
        return Err(AppError::NotFound);
    }

    let row = sqlx::query_as!(
        EventRow,
        r#"
        INSERT INTO events
            (calendar_id, created_by, title, description, location,
             start_at, end_at, tz, rrule, all_day)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, calendar_id, title, description, location,
                  start_at, end_at, tz, rrule, all_day,
                  created_at, updated_at
        "#,
        input.calendar_id,
        user.id,
        title,
        input.description,
        input.location,
        input.start_at,
        input.end_at,
        input.tz,
        input.rrule,
        input.all_day,
    )
    .fetch_one(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateInput>,
) -> AppResult<Json<EventRow>> {
    if let Some(ref tz) = input.tz
        && tz.parse::<chrono_tz::Tz>().is_err()
    {
        return Err(AppError::Validation("tz must be a valid IANA zone".into()));
    }
    // rrule field: Some(Some(x)) = set, Some(None) = clear, None = leave.
    let (set_rrule, clear_rrule) = match input.rrule {
        Some(Some(v)) => (Some(v), false),
        Some(None) => (None, true),
        None => (None, false),
    };

    let row = sqlx::query_as!(
        EventRow,
        r#"
        UPDATE events e
        SET title       = COALESCE($3, title),
            description = COALESCE($4, description),
            location    = COALESCE($5, location),
            start_at    = COALESCE($6, start_at),
            end_at      = COALESCE($7, end_at),
            tz          = COALESCE($8, tz),
            rrule       = CASE WHEN $10 THEN NULL ELSE COALESCE($9, rrule) END,
            all_day     = COALESCE($11, all_day)
        FROM calendars c
        LEFT JOIN calendar_shares cs ON cs.calendar_id = c.id AND cs.user_id = $2
        WHERE e.id = $1
          AND c.id = e.calendar_id
          AND (c.owner_id = $2 OR cs.permission = 'edit')
        RETURNING e.id, e.calendar_id, e.title, e.description, e.location,
                  e.start_at, e.end_at, e.tz, e.rrule, e.all_day,
                  e.created_at, e.updated_at
        "#,
        id,
        user.id,
        input.title,
        input.description,
        input.location,
        input.start_at,
        input.end_at,
        input.tz,
        set_rrule,
        clear_rrule,
        input.all_day,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

async fn delete(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let res = sqlx::query!(
        r#"DELETE FROM events e
           USING calendars c
           LEFT JOIN calendar_shares cs ON cs.calendar_id = c.id AND cs.user_id = $2
           WHERE e.id = $1
             AND c.id = e.calendar_id
             AND (c.owner_id = $2 OR cs.permission = 'edit')"#,
        id,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
