//! Comment type only — the handlers live in `routes::cards` so we share
//! the card → list → board lookup that gates RBAC.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize, FromRow)]
pub struct Comment {
    pub id: Uuid,
    pub card_id: Uuid,
    pub author_id: Uuid,
    pub author_email: String,
    pub author_name: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
