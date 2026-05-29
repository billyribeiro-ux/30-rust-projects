pub mod auth;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use axum::http::{HeaderValue, Method};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use state::AppState;

pub fn build_app(state: AppState, cors_origin: &str) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(cors_origin.parse::<HeaderValue>().expect("origin"))
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api/interviews", routes::interviews::router())
        .nest("/api/exec", routes::executions::router())
        .nest("/ws", routes::ws::router())
        .nest("/saml", routes::saml::router())
        .route("/healthz", axum::routing::get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health() -> &'static str {
    "ok"
}

pub fn make_hubs() -> state::Hubs {
    Arc::new(Mutex::new(HashMap::new()))
}
