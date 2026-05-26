use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};

const ALLOWED_KINDS: &[&str] = &["asset", "liability", "income", "expense", "equity"];
const ALLOWED_COLORS: &[&str] = &[
    "#4F46E5", "#059669", "#D97706", "#DC2626", "#0284C7", "#7C3AED", "#DB2777", "#0F766E",
];

#[derive(Debug, Serialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub balance_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccount {
    pub name: String,
    pub kind: String,
    pub color: String,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::delete(delete))
}

async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<Account>>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            a.id, a.name, a.kind, a.color, a.created_at,
            COALESCE((SELECT SUM(amount_minor) FROM postings WHERE account_id = a.id), 0) AS "balance!: i64"
        FROM accounts a
        ORDER BY a.created_at ASC
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| Account {
                id: r.id.expect("id is non-null primary key"),
                name: r.name,
                kind: r.kind,
                color: r.color,
                created_at: parse_ts(&r.created_at),
                balance_minor: r.balance,
            })
            .collect(),
    ))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateAccount>,
) -> AppResult<(StatusCode, Json<Account>)> {
    let mut errors = Vec::new();
    let name = match normalize_name(&payload.name) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            String::new()
        }
    };
    let kind = match normalize_kind(&payload.kind) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            String::new()
        }
    };
    let color = match normalize_color(&payload.color) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            String::new()
        }
    };
    if !errors.is_empty() {
        return Err(AppError::Fields(errors));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        "INSERT INTO accounts (id, name, kind, color, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        id,
        name,
        kind,
        color,
        now_str,
    )
    .execute(&pool)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(dbe) = &err
            && dbe.message().contains("UNIQUE constraint failed")
        {
            return AppError::Fields(vec![FieldError {
                field: "name".into(),
                message: "an account with that name already exists".into(),
            }]);
        }
        AppError::Database(err)
    })?;

    Ok((
        StatusCode::CREATED,
        Json(Account {
            id,
            name,
            kind,
            color,
            created_at: now,
            balance_minor: 0,
        }),
    ))
}

async fn delete(State(pool): State<SqlitePool>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM accounts WHERE id = ?1", id)
        .execute(&pool)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(dbe) = &err
                && dbe.message().contains("FOREIGN KEY constraint failed")
            {
                return AppError::Fields(vec![FieldError {
                    field: "id".into(),
                    message: "cannot delete: account has postings".into(),
                }]);
            }
            AppError::Database(err)
        })?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn normalize_name(raw: &str) -> Result<String, FieldError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(FieldError {
            field: "name".into(),
            message: "must not be empty".into(),
        });
    }
    if trimmed.chars().count() > 60 {
        return Err(FieldError {
            field: "name".into(),
            message: "must be 60 chars or fewer".into(),
        });
    }
    Ok(trimmed.to_string())
}

fn normalize_kind(raw: &str) -> Result<String, FieldError> {
    let trimmed = raw.trim().to_lowercase();
    if !ALLOWED_KINDS.contains(&trimmed.as_str()) {
        return Err(FieldError {
            field: "kind".into(),
            message: format!("must be one of: {}", ALLOWED_KINDS.join(", ")),
        });
    }
    Ok(trimmed)
}

fn normalize_color(raw: &str) -> Result<String, FieldError> {
    let trimmed = raw.trim().to_uppercase();
    if !ALLOWED_COLORS.contains(&trimmed.as_str()) {
        return Err(FieldError {
            field: "color".into(),
            message: "must be one of the palette colors".into(),
        });
    }
    Ok(trimmed)
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
