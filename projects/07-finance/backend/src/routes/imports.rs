use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::ledger::decimal_to_minor;

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    /// CSV body. Expected columns: date,description,amount,counterparty_account
    /// `amount` is positive when the asset_account receives money, negative
    /// when it pays out. The `counterparty_account` is matched by name.
    pub csv: String,
    /// The account name the CSV is FOR (the asset side). Required so we know
    /// where the money came from / went to.
    pub asset_account_name: String,
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/", post(import))
}

async fn import(
    State(pool): State<SqlitePool>,
    Json(req): Json<ImportRequest>,
) -> AppResult<Json<ImportResponse>> {
    // Lookup the asset account
    let asset_name = req.asset_account_name.trim();
    if asset_name.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "asset_account_name".into(),
            message: "required".into(),
        }]));
    }
    let asset_row = sqlx::query!("SELECT id FROM accounts WHERE name = ?1", asset_name)
        .fetch_optional(&pool)
        .await?;
    let asset_id = asset_row
        .ok_or_else(|| {
            AppError::Fields(vec![FieldError {
                field: "asset_account_name".into(),
                message: format!("no account named '{}'", asset_name),
            }])
        })?
        .id
        .expect("id is non-null primary key");

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(req.csv.as_bytes());

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    let mut tx = pool.begin().await?;
    for (i, rec) in rdr.records().enumerate() {
        let line = i + 2; // +1 for header, +1 to be human-friendly
        let row = match rec {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("line {line}: parse error: {e}"));
                skipped += 1;
                continue;
            }
        };

        // Expected columns: date, description, amount, counterparty_account
        let date_str = row.get(0).unwrap_or("").trim();
        let description = row.get(1).unwrap_or("").trim();
        let amount_str = row.get(2).unwrap_or("").trim();
        let counterparty = row.get(3).unwrap_or("").trim();

        let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => {
                errors.push(format!(
                    "line {line}: invalid date '{date_str}' (expected YYYY-MM-DD)"
                ));
                skipped += 1;
                continue;
            }
        };

        let amount_dec: Decimal = match amount_str.parse() {
            Ok(d) => d,
            Err(_) => {
                errors.push(format!("line {line}: invalid amount '{amount_str}'"));
                skipped += 1;
                continue;
            }
        };
        let amount_minor = match decimal_to_minor(amount_dec) {
            Ok(m) => m,
            Err(e) => {
                errors.push(format!("line {line}: {e}"));
                skipped += 1;
                continue;
            }
        };
        if amount_minor == 0 {
            errors.push(format!("line {line}: amount is zero, skipping"));
            skipped += 1;
            continue;
        }

        // Look up (or skip) the counterparty
        let cp_row = sqlx::query!("SELECT id FROM accounts WHERE name = ?1", counterparty)
            .fetch_optional(&mut *tx)
            .await?;
        let cp_id = match cp_row {
            Some(r) => r.id.expect("id is non-null primary key"),
            None => {
                errors.push(format!(
                    "line {line}: no account named '{counterparty}' — skipping"
                ));
                skipped += 1;
                continue;
            }
        };

        let tx_id = Uuid::new_v4().to_string();
        let occurred_at: DateTime<Utc> =
            date.and_hms_opt(0, 0, 0).expect("midnight valid").and_utc();
        let now = Utc::now();
        let occurred_str = format_ts(occurred_at);
        let now_str = format_ts(now);

        sqlx::query!(
            "INSERT INTO transactions (id, description, occurred_at, created_at) VALUES (?1, ?2, ?3, ?4)",
            tx_id,
            description,
            occurred_str,
            now_str,
        )
        .execute(&mut *tx)
        .await?;

        // Asset side: amount as-is. Counterparty side: negative.
        let pid1 = Uuid::new_v4().to_string();
        sqlx::query!(
            "INSERT INTO postings (id, transaction_id, account_id, amount_minor) VALUES (?1, ?2, ?3, ?4)",
            pid1,
            tx_id,
            asset_id,
            amount_minor,
        )
        .execute(&mut *tx)
        .await?;

        let neg_amount = -amount_minor;
        let pid2 = Uuid::new_v4().to_string();
        sqlx::query!(
            "INSERT INTO postings (id, transaction_id, account_id, amount_minor) VALUES (?1, ?2, ?3, ?4)",
            pid2,
            tx_id,
            cp_id,
            neg_amount,
        )
        .execute(&mut *tx)
        .await?;

        imported += 1;
    }
    tx.commit().await?;

    Ok(Json(ImportResponse {
        imported,
        skipped,
        errors,
    }))
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}
