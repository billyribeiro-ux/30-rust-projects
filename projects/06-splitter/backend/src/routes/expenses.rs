use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::splitter::{ShareInput, ShareOutput, SplitKind, split};

#[derive(Debug, Serialize)]
pub struct Expense {
    pub id: String,
    pub payer_id: String,
    pub amount_cents: i64,
    pub description: String,
    pub paid_at: DateTime<Utc>,
    pub split_kind: SplitKind,
    pub created_at: DateTime<Utc>,
    pub shares: Vec<ShareOutput>,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpense {
    pub payer_id: String,
    pub amount_cents: i64,
    pub description: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub split_kind: SplitKind,
    pub shares: Vec<ShareInput>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::delete(delete))
}

async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<Expense>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, payer_id, amount_cents, description, paid_at, split_kind, created_at
        FROM expenses
        ORDER BY paid_at DESC, created_at DESC
        "#
    )
    .fetch_all(&pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row.id.expect("id is non-null primary key");
        let shares = fetch_shares(&pool, &id).await?;
        let split_kind = parse_kind(&row.split_kind);
        out.push(Expense {
            id,
            payer_id: row.payer_id,
            amount_cents: row.amount_cents,
            description: row.description,
            paid_at: parse_ts(&row.paid_at),
            split_kind,
            created_at: parse_ts(&row.created_at),
            shares,
        });
    }
    Ok(Json(out))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateExpense>,
) -> AppResult<(StatusCode, Json<Expense>)> {
    let mut errors = Vec::new();

    // payer exists?
    let payer_ok = sqlx::query!("SELECT id FROM members WHERE id = ?1", payload.payer_id)
        .fetch_optional(&pool)
        .await?;
    if payer_ok.is_none() {
        errors.push(FieldError {
            field: "payer_id".into(),
            message: "unknown payer".into(),
        });
    }

    if payload.amount_cents <= 0 {
        errors.push(FieldError {
            field: "amount_cents".into(),
            message: "must be greater than 0".into(),
        });
    }
    if payload.amount_cents > 1_000_000_000 {
        errors.push(FieldError {
            field: "amount_cents".into(),
            message: "amount is too large".into(),
        });
    }
    let description = normalize_description(payload.description.as_deref().unwrap_or(""))
        .unwrap_or_else(|e| {
            errors.push(e);
            String::new()
        });

    if !errors.is_empty() {
        return Err(AppError::Fields(errors));
    }

    let computed = split(payload.amount_cents, payload.split_kind, &payload.shares)?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let paid_at = payload.paid_at.unwrap_or(now);
    let now_str = format_ts(now);
    let paid_str = format_ts(paid_at);
    let kind_str = kind_str(payload.split_kind);

    let mut tx = pool.begin().await?;
    sqlx::query!(
        r#"
        INSERT INTO expenses (id, payer_id, amount_cents, description, paid_at, split_kind, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        id,
        payload.payer_id,
        payload.amount_cents,
        description,
        paid_str,
        kind_str,
        now_str,
    )
    .execute(&mut *tx)
    .await?;

    for share in &computed {
        sqlx::query!(
            "INSERT INTO expense_shares (expense_id, member_id, share_cents) VALUES (?1, ?2, ?3)",
            id,
            share.member_id,
            share.share_cents,
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(dbe) = &err
                && dbe.message().contains("FOREIGN KEY constraint failed")
            {
                return AppError::Fields(vec![FieldError {
                    field: "shares".into(),
                    message: format!("unknown member id: {}", share.member_id),
                }]);
            }
            AppError::Database(err)
        })?;
    }
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(Expense {
            id,
            payer_id: payload.payer_id,
            amount_cents: payload.amount_cents,
            description,
            paid_at,
            split_kind: payload.split_kind,
            created_at: now,
            shares: computed,
        }),
    ))
}

async fn delete(State(pool): State<SqlitePool>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM expenses WHERE id = ?1", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn fetch_shares(pool: &SqlitePool, expense_id: &str) -> AppResult<Vec<ShareOutput>> {
    let rows = sqlx::query!(
        "SELECT member_id, share_cents FROM expense_shares WHERE expense_id = ?1",
        expense_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ShareOutput {
            member_id: r.member_id,
            share_cents: r.share_cents,
        })
        .collect())
}

fn normalize_description(raw: &str) -> Result<String, FieldError> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > 200 {
        return Err(FieldError {
            field: "description".into(),
            message: "must be 200 chars or fewer".into(),
        });
    }
    Ok(trimmed.to_string())
}

fn kind_str(k: SplitKind) -> &'static str {
    match k {
        SplitKind::Equal => "equal",
        SplitKind::Exact => "exact",
        SplitKind::Percent => "percent",
    }
}

fn parse_kind(raw: &str) -> SplitKind {
    match raw {
        "exact" => SplitKind::Exact,
        "percent" => SplitKind::Percent,
        _ => SplitKind::Equal,
    }
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
