use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::ledger::{PostingInput, validate_postings};

#[derive(Debug, Serialize)]
pub struct Posting {
    pub id: String,
    pub account_id: String,
    pub amount_minor: i64,
}

#[derive(Debug, Serialize)]
pub struct Transaction {
    pub id: String,
    pub description: String,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub postings: Vec<Posting>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTransaction {
    pub description: Option<String>,
    pub occurred_at: Option<DateTime<Utc>>,
    pub postings: Vec<PostingInput>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::delete(delete))
}

async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<Transaction>>> {
    let tx_rows = sqlx::query!(
        r#"
        SELECT id, description, occurred_at, created_at
        FROM transactions
        ORDER BY occurred_at DESC, created_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let mut out = Vec::with_capacity(tx_rows.len());
    for row in tx_rows {
        let id = row.id.expect("id is non-null primary key");
        let postings = fetch_postings(&pool, &id).await?;
        out.push(Transaction {
            id,
            description: row.description,
            occurred_at: parse_ts(&row.occurred_at),
            created_at: parse_ts(&row.created_at),
            postings,
        });
    }
    Ok(Json(out))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateTransaction>,
) -> AppResult<(StatusCode, Json<Transaction>)> {
    validate_postings(&payload.postings)?;

    let description = payload
        .description
        .as_deref()
        .map(|s| s.trim())
        .unwrap_or("");
    if description.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "description".into(),
            message: "must be 200 chars or fewer".into(),
        }]));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let occurred_at = payload.occurred_at.unwrap_or(now);
    let now_str = format_ts(now);
    let occurred_str = format_ts(occurred_at);

    let mut tx = pool.begin().await?;
    sqlx::query!(
        "INSERT INTO transactions (id, description, occurred_at, created_at) VALUES (?1, ?2, ?3, ?4)",
        id,
        description,
        occurred_str,
        now_str,
    )
    .execute(&mut *tx)
    .await?;

    let mut postings_out = Vec::with_capacity(payload.postings.len());
    for p in &payload.postings {
        let posting_id = Uuid::new_v4().to_string();
        sqlx::query!(
            "INSERT INTO postings (id, transaction_id, account_id, amount_minor) VALUES (?1, ?2, ?3, ?4)",
            posting_id,
            id,
            p.account_id,
            p.amount_minor,
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(dbe) = &err
                && dbe.message().contains("FOREIGN KEY constraint failed")
            {
                return AppError::Fields(vec![FieldError {
                    field: "postings".into(),
                    message: format!("unknown account_id: {}", p.account_id),
                }]);
            }
            AppError::Database(err)
        })?;
        postings_out.push(Posting {
            id: posting_id,
            account_id: p.account_id.clone(),
            amount_minor: p.amount_minor,
        });
    }
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(Transaction {
            id,
            description: description.to_string(),
            occurred_at,
            created_at: now,
            postings: postings_out,
        }),
    ))
}

async fn delete(State(pool): State<SqlitePool>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM transactions WHERE id = ?1", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn fetch_postings(pool: &SqlitePool, transaction_id: &str) -> AppResult<Vec<Posting>> {
    let rows = sqlx::query!(
        "SELECT id, account_id, amount_minor FROM postings WHERE transaction_id = ?1",
        transaction_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Posting {
            id: r.id.expect("id is non-null primary key"),
            account_id: r.account_id,
            amount_minor: r.amount_minor,
        })
        .collect())
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
