//! WebSocket collab — last-write-wins (LWW) document broadcast.
//!
//! Each connected participant receives every other's edit verbatim.
//! Edits are JSON: `{"kind":"code","value":"…"}` or
//! `{"kind":"cursor","line":N,"col":N}`. The server persists code
//! changes to `interviews.code` (debounced via a Tokio mutex).
//!
//! This is NOT a CRDT — two simultaneous typists step on each other.
//! Per LESSON.md §A1 the upgrade to `yrs` is the future-work path.

use axum::Router;
use axum::extract::{
    Path, State, WebSocketUpgrade,
    ws::{Message, WebSocket},
};
use axum::response::IntoResponse;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/interview/{id}", get(socket))
}

async fn socket(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle(socket, s, id))
}

async fn handle(socket: WebSocket, s: AppState, id: Uuid) {
    // Get or create the per-interview broadcaster.
    let tx = {
        let mut hubs = s.hubs.lock().await;
        hubs.entry(id)
            .or_insert_with(|| broadcast::channel::<String>(256).0)
            .clone()
    };
    let mut rx = tx.subscribe();
    let (mut sink, mut stream) = socket.split();

    // Pipe broadcast → this socket.
    let forward = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Read from socket → broadcast + persist code on `kind:code`.
    while let Some(Ok(msg)) = stream.next().await {
        if let Message::Text(text) = msg {
            let _ = tx.send(text.to_string());
            // Best-effort persist on `kind:code`.
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text)
                && v.get("kind").and_then(|k| k.as_str()) == Some("code")
                && let Some(code) = v.get("value").and_then(|c| c.as_str())
            {
                let _ = sqlx::query!(
                    "UPDATE interviews SET code = $2, updated_at = now() WHERE id = $1",
                    id,
                    code
                )
                .execute(&s.pool)
                .await;
            }
        }
    }
    forward.abort();

    // Drop the broadcaster if we were the last subscriber.
    let mut hubs = s.hubs.lock().await;
    if let Some(t) = hubs.get(&id)
        && t.receiver_count() == 0
    {
        hubs.remove(&id);
    }
}
