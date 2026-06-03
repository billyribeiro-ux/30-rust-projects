//! `SET LOCAL` discipline for RLS-aware queries.
//!
//! Call `enter_tenant(tx, tenant_id, user_id)` at the start of every
//! transaction that touches RLS-protected tables. The settings live
//! for the duration of the transaction (`SET LOCAL`), so the next
//! transaction on the same connection sees them cleared — no chance
//! of leaking a previous request's tenant context onto the connection
//! pool.

use sqlx::AssertSqlSafe;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::AppResult;

pub async fn enter_tenant<'c>(
    tx: &mut Transaction<'c, Postgres>,
    tenant_id: Uuid,
    user_id: Option<Uuid>,
) -> AppResult<()> {
    // UUIDs are safe to interpolate (no quote/escape characters), but we
    // keep the formatter defensive anyway by validating via the Uuid type.
    sqlx::query(AssertSqlSafe(format!("SET LOCAL app.tenant_id = '{tenant_id}'")))
        .execute(&mut **tx)
        .await?;
    if let Some(uid) = user_id {
        sqlx::query(AssertSqlSafe(format!("SET LOCAL app.user_id = '{uid}'")))
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
