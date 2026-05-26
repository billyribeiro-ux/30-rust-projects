//! Message scrollback.
//!
//! `GET /api/rooms/{slug}/messages?before=<ts>&limit=50` — newest-first
//! paged scroll. Cursor is the ISO timestamp of the oldest message
//! currently rendered; the server returns the next batch older than
//! that. `limit` is clamped to 1..=200.
//!
//! Sending happens over the WebSocket (`routes/ws.rs`), which writes
//! to the DB AND broadcasts to the room channel. Splitting "send" (WS)
//! from "scroll" (HTTP) keeps the WS protocol minimal — it's really
//! only an event-fanout pipe.

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/{slug}/messages", get(list))
}

#[derive(Serialize)]
pub struct Message {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ListParams {
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

async fn list(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
    Query(p): Query<ListParams>,
) -> AppResult<Json<Vec<Message>>> {
    // Verify membership via the same join we'd do in rooms::read — if the
    // user isn't in the room, the join returns no rows and we surface 404.
    let room = sqlx::query!(
        r#"SELECT r.id FROM rooms r
           JOIN room_members m ON m.room_id = r.id
           WHERE r.slug = $1 AND m.user_id = $2"#,
        slug,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let limit = p.limit.unwrap_or(50).clamp(1, 200);
    let before = p
        .before
        .unwrap_or_else(|| Utc::now() + chrono::Duration::days(1));

    let rows = sqlx::query_as!(
        Message,
        r#"
        SELECT m.id, m.room_id, m.user_id, u.name AS author, m.body, m.created_at
        FROM messages m
        JOIN users u ON u.id = m.user_id
        WHERE m.room_id = $1 AND m.created_at < $2
        ORDER BY m.created_at DESC
        LIMIT $3
        "#,
        room.id,
        before,
        limit,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}
