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
        .allow_origin(cors_origin.parse::<HeaderValue>().expect("origin"))
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
        .nest("/api/instructor", routes::instructors::router())
        .nest("/api/courses", routes::courses::router())
        .nest("/api/enroll", routes::enrollments::router())
        .nest("/api/stripe", routes::stripe::router())
        .route("/healthz", axum::routing::get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health() -> &'static str {
    "ok"
}
