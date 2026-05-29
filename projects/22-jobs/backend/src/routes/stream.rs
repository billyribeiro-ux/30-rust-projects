//! SSE stream of `JobEvent`s. The dashboard subscribes and updates
//! status badges live, without polling.

use axum::Router;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use futures_core::Stream;
use std::convert::Infallible;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::auth::session::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/jobs", get(stream_jobs))
}

async fn stream_jobs(
    State(s): State<AppState>,
    _user: AuthUser,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = s.events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(ev) => Some(Ok(Event::default()
            .event("job")
            .json_data(&ev)
            .unwrap_or_default())),
        // The receiver lagged — emit a "lag" event so the client knows
        // to refetch the full list rather than trust live state.
        Err(_) => Some(Ok(Event::default().event("lag").data("0"))),
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
