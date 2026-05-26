//! WebSocket route — the heart of project 13.
//!
//! Connection lifecycle:
//!
//! 1. Client hits GET /api/rooms/{slug}/ws with the session cookie.
//!    `AuthUser` extractor verifies the cookie BEFORE the upgrade. If
//!    auth fails, we 401 before the protocol switch — no leaky open
//!    socket on rejected auth.
//! 2. We verify the user is a member of the room (same SQL as messages).
//! 3. We upgrade. The handler splits into a `recv` task and a `send`
//!    task that share the room's broadcast channel. `recv` reads frames
//!    from the browser, parses them as `ClientMsg`, persists, and
//!    broadcasts to the room. `send` listens on the broadcast receiver
//!    and writes anything it sees back to the browser.
//! 4. On disconnect we drop the presence entry and broadcast a `Left`
//!    event so other clients update their roster.
//!
//! The two-tasks-with-`tokio::select` pattern is the standard axum WS
//! shape. It lets a single closed-by-peer event tear down both halves
//! deterministically.

use axum::Router;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::{AppState, RoomEvent};

pub fn router() -> Router<AppState> {
    Router::new().route("/{slug}/ws", get(ws_handler))
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Send { body: String },
    // Future: typing indicators, read receipts. The tagged-enum shape
    // keeps the wire format forward-compatible.
}

async fn ws_handler(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
    ws: WebSocketUpgrade,
) -> AppResult<impl IntoResponse> {
    // Membership check happens BEFORE the upgrade so auth-rejection
    // returns a normal HTTP response, not a WS-handshake failure.
    let row = sqlx::query!(
        r#"SELECT r.id, r.slug, u.name AS author
           FROM rooms r
           JOIN room_members m ON m.room_id = r.id
           JOIN users u ON u.id = $2
           WHERE r.slug = $1 AND m.user_id = $2"#,
        slug,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let room_id = row.id;
    let author = row.author;

    Ok(ws.on_upgrade(move |socket| handle(socket, s, room_id, user.id, author)))
}

async fn handle(socket: WebSocket, state: AppState, room_id: Uuid, user_id: Uuid, author: String) {
    let tx = state.hub.sender(room_id);
    let mut rx: broadcast::Receiver<RoomEvent> = tx.subscribe();

    // Presence: add to the room's set and broadcast a join.
    state
        .hub
        .presence
        .entry(room_id)
        .or_default()
        .insert(user_id);
    let _ = tx.send(RoomEvent::Joined {
        user_id,
        author: author.clone(),
    });

    let (mut ws_tx, mut ws_rx) = socket.split();

    // ---- send task: broadcast → websocket ----
    let send_tx = tx.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            let json = match serde_json::to_string(&ev) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if ws_tx.send(WsMessage::Text(json.into())).await.is_err() {
                break;
            }
        }
        // signal recv task to stop if this side dies
        drop(send_tx);
    });

    // ---- recv task: websocket → DB + broadcast ----
    let recv_pool = state.pool.clone();
    let recv_tx = tx.clone();
    let recv_author = author.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                WsMessage::Text(t) => {
                    let Ok(parsed) = serde_json::from_str::<ClientMsg>(&t) else {
                        continue;
                    };
                    match parsed {
                        ClientMsg::Send { body } => {
                            let body = body.trim().to_string();
                            if body.is_empty() || body.chars().count() > 2000 {
                                continue;
                            }
                            let rec = sqlx::query!(
                                r#"INSERT INTO messages (room_id, user_id, body)
                                   VALUES ($1, $2, $3)
                                   RETURNING id, created_at"#,
                                room_id,
                                user_id,
                                body,
                            )
                            .fetch_one(&recv_pool)
                            .await;
                            if let Ok(rec) = rec {
                                let _ = recv_tx.send(RoomEvent::Message {
                                    id: rec.id,
                                    room_id,
                                    user_id,
                                    author: recv_author.clone(),
                                    body,
                                    created_at: rec.created_at,
                                });
                            }
                        }
                    }
                }
                WsMessage::Close(_) => break,
                _ => {} // ignore binary/ping/pong (axum handles pings)
            }
        }
    });

    // Wait for either task to finish — that means the peer is gone or the
    // broadcast channel was closed. Either way, tear down the other half.
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    // Presence: drop and broadcast left.
    if let Some(set) = state.hub.presence.get(&room_id) {
        set.remove(&user_id);
    }
    let _ = tx.send(RoomEvent::Left { user_id, author });
}
