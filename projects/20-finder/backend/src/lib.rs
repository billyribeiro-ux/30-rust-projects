//! Library crate so the OAuth + PKCE primitives can be exercised by the
//! integration tests in `tests/` without spinning up the full Axum app,
//! and so `main.rs` is a thin shim over `build_app`.

pub mod auth;
pub mod db;
pub mod error;
pub mod oauth;
pub mod routes;
pub mod state;

use axum::Router;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use state::AppState;

/// Build the full application router for `state`.
///
/// `cors_origin` is the single allowed browser origin. We never use `*`
/// because the API serves credentialed (cookie) requests and a wildcard
/// origin is incompatible with `Access-Control-Allow-Credentials: true`.
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
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::HeaderName::from_static("x-test-token"),
        ])
        .allow_credentials(true);

    Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api/auth/oauth", routes::oauth::router())
        .nest("/api/places", routes::places::router())
        .nest("/api/reviews", routes::reviews::router())
        .nest("/api/recs", routes::recs::router())
        .nest("/api/test", routes::test_only::router())
        .route("/healthz", axum::routing::get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health() -> &'static str {
    "ok"
}
