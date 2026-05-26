mod db;
mod error;
mod routes;
mod streaks;

use axum::Router;
use axum::http::{HeaderValue, Method};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "habits_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://habits.db".to_string());
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5175".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3002);

    let pool = db::connect(&db_url).await?;
    tracing::info!(db = %db_url, "database ready");

    let cors = CorsLayer::new()
        .allow_origin(frontend_origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .nest("/api/habits", routes::habits::router())
        .route("/healthz", axum::routing::get(health))
        .with_state(pool)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("ctrl-c received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
