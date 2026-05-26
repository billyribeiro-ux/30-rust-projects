use dashmap::{DashMap, DashSet};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::email::Mailer;

/// A single chat event broadcast inside one room. The `ChatHub` owns the
/// fan-out channels; every WebSocket handler subscribes to its room's
/// receiver and forwards anything it sees down to the browser.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RoomEvent {
    /// A new chat message arrived.
    Message {
        id: Uuid,
        room_id: Uuid,
        user_id: Uuid,
        author: String,
        body: String,
        created_at: chrono::DateTime<chrono::Utc>,
    },
    /// Someone joined the room (presence-up).
    Joined { user_id: Uuid, author: String },
    /// Someone left the room (presence-down).
    Left { user_id: Uuid, author: String },
}

/// Per-room broadcast channel + presence set. One entry per room.
///
/// `tokio::sync::broadcast` is a multi-producer, multi-consumer channel where
/// every receiver sees every message (vs. mpsc where each message goes to
/// exactly one consumer). That's exactly what a chat room needs: one user
/// sends, everyone in the room receives.
///
/// Capacity 256: if a slow client's receiver lags by more than 256 messages,
/// `recv()` returns `RecvError::Lagged` and the handler can either reconnect
/// or drop the connection. We chose drop — see routes/ws.rs.
pub struct ChatHub {
    pub channels: DashMap<Uuid, broadcast::Sender<RoomEvent>>,
    pub presence: DashMap<Uuid, DashSet<Uuid>>, // room_id → set<user_id>
}

impl ChatHub {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
            presence: DashMap::new(),
        }
    }

    /// Get-or-create the broadcast sender for a room. Idempotent.
    pub fn sender(&self, room_id: Uuid) -> broadcast::Sender<RoomEvent> {
        self.channels
            .entry(room_id)
            .or_insert_with(|| broadcast::channel(256).0)
            .clone()
    }
}

impl Default for ChatHub {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub mailer: Arc<Mailer>,
    pub public_url: String,
    pub secure_cookies: bool,
    pub hub: Arc<ChatHub>,
}
