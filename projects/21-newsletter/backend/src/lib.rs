//! Library crate so integration tests can exercise routes/types
//! without booting the binary. `main.rs` is a thin shim over
//! `build_app`.

pub mod auth;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;
pub mod stripe;

use axum::Router;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use state::AppState;

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
        .nest("/api/admin", routes::admin::router())
        .nest("/api/posts", routes::posts::router())
        .nest("/api/subscribe", routes::subscribe::router())
        .nest("/api/stripe", routes::stripe::router())
        .nest("/api/magic", routes::magic::router())
        .merge(routes::feed::router())
        .route("/healthz", axum::routing::get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health() -> &'static str {
    "ok"
}
