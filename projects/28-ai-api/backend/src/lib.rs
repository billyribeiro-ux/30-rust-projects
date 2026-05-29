pub mod auth;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use axum::http::{HeaderValue, Method};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use url::Url;
use webauthn_rs::WebauthnBuilder;

pub use state::AppState;

pub fn build_webauthn(rp_id: &str, rp_name: &str, origin: &str) -> Arc<webauthn_rs::Webauthn> {
    let url = Url::parse(origin).expect("WEBAUTHN_ORIGIN must be a valid URL");
    let b = WebauthnBuilder::new(rp_id, &url)
        .expect("webauthn builder")
        .rp_name(rp_name);
    Arc::new(b.build().expect("webauthn build"))
}

pub fn build_app(state: AppState, cors_origin: &str) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(cors_origin.parse::<HeaderValue>().expect("origin"))
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api/passkeys", routes::passkeys::router())
        .nest("/api/keys", routes::keys::router())
        .nest("/api/usage", routes::usage::router())
        .nest("/v1/inference", routes::inference::router())
        .route("/healthz", axum::routing::get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health() -> &'static str {
    "ok"
}
