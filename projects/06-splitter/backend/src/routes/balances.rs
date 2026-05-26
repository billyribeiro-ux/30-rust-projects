use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::error::AppResult;

#[derive(Debug, Serialize)]
pub struct Balance {
    pub member_id: String,
    pub name: String,
    pub paid_cents: i64,
    pub owed_cents: i64,
    /// Positive = others owe this member. Negative = this member owes the group.
    pub net_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct Settlement {
    pub from: String,
    pub to: String,
    pub cents: i64,
}

#[derive(Debug, Serialize)]
pub struct BalancesResponse {
    pub balances: Vec<Balance>,
    pub settlements: Vec<Settlement>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/", get(get_balances))
}

async fn get_balances(State(pool): State<SqlitePool>) -> AppResult<Json<BalancesResponse>> {
    let members = sqlx::query!("SELECT id, name FROM members ORDER BY created_at ASC")
        .fetch_all(&pool)
        .await?;

    let paid_rows = sqlx::query!(
        r#"
        SELECT payer_id, COALESCE(SUM(amount_cents), 0) AS "paid!: i64"
        FROM expenses
        GROUP BY payer_id
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let owed_rows = sqlx::query!(
        r#"
        SELECT member_id, COALESCE(SUM(share_cents), 0) AS "owed!: i64"
        FROM expense_shares
        GROUP BY member_id
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let paid_map: HashMap<String, i64> = paid_rows
        .into_iter()
        .map(|r| (r.payer_id, r.paid))
        .collect();
    let owed_map: HashMap<String, i64> = owed_rows
        .into_iter()
        .map(|r| (r.member_id, r.owed))
        .collect();

    let mut balances: Vec<Balance> = members
        .into_iter()
        .map(|m| {
            let id = m.id.expect("id is non-null primary key");
            let paid = paid_map.get(&id).copied().unwrap_or(0);
            let owed = owed_map.get(&id).copied().unwrap_or(0);
            Balance {
                member_id: id,
                name: m.name,
                paid_cents: paid,
                owed_cents: owed,
                net_cents: paid - owed,
            }
        })
        .collect();

    balances.sort_by(|a, b| b.net_cents.cmp(&a.net_cents));
    let settlements = compute_settlements(&balances);

    Ok(Json(BalancesResponse {
        balances,
        settlements,
    }))
}

/// Greedy minimum-transaction settlement: pair the largest creditor with the
/// largest debtor, pay off the smaller of the two, repeat. Produces at most
/// (N-1) transactions for N members.
fn compute_settlements(balances: &[Balance]) -> Vec<Settlement> {
    let mut creditors: Vec<(String, i64)> = balances
        .iter()
        .filter(|b| b.net_cents > 0)
        .map(|b| (b.member_id.clone(), b.net_cents))
        .collect();
    let mut debtors: Vec<(String, i64)> = balances
        .iter()
        .filter(|b| b.net_cents < 0)
        .map(|b| (b.member_id.clone(), -b.net_cents))
        .collect();

    let mut out = Vec::new();
    while let (Some(c), Some(d)) = (creditors.last_mut(), debtors.last_mut()) {
        let amount = c.1.min(d.1);
        if amount == 0 {
            break;
        }
        out.push(Settlement {
            from: d.0.clone(),
            to: c.0.clone(),
            cents: amount,
        });
        c.1 -= amount;
        d.1 -= amount;
        if c.1 == 0 {
            creditors.pop();
        }
        if d.1 == 0 {
            debtors.pop();
        }
    }
    out
}
