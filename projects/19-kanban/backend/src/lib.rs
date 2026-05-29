//! Library surface for integration tests + the binary.
//!
//! `main.rs` is a thin shim over [`build_app`]; the integration tests import the
//! same builder so they exercise the exact routing and middleware the server
//! runs in production. Keeping the wiring in one place is the only way the test
//! suite can honestly claim "this is what ships".

pub mod auth;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use state::AppState;

/// Build the full application router for `state`.
///
/// `cors_origin` is the single allowed browser origin. We never use `*` because
/// the API serves credentialed (cookie) requests and a wildcard origin is
/// incompatible with `Access-Control-Allow-Credentials: true`.
pub fn build_app(state: AppState, cors_origin: &str) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            cors_origin
                .parse::<HeaderValue>()
                .expect("FRONTEND_ORIGIN must be a valid header value"),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api/boards", routes::boards::router())
        .nest("/api/lists", routes::lists::router())
        .nest("/api/cards", routes::cards::router())
        .route("/healthz", axum::routing::get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health() -> &'static str {
    "ok"
}
