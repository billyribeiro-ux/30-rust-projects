mod db;
mod error;
mod routes;
mod signed;
mod state;
mod uploads;

use axum::Router;
use axum::http::{HeaderValue, Method};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::signed::Signer;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "recipes_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://recipes.db".to_string());
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5182".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3009);
    let upload_dir =
        PathBuf::from(std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()));
    tokio::fs::create_dir_all(&upload_dir).await?;

    // Signing key. In real life this comes from a secret manager. The
    // fallback here is fine for dev because share URLs are not security
    // boundaries on a single-user app — they're convenience tokens.
    let secret = std::env::var("SHARE_SECRET")
        .unwrap_or_else(|_| "dev-share-secret-please-override-in-prod".into());
    if secret.len() < 16 {
        return Err("SHARE_SECRET must be at least 16 bytes".into());
    }
    let signer = Signer::new(secret.into_bytes());

    let pool = db::connect(&db_url).await?;
    tracing::info!(db = %db_url, "database ready");

    let state = AppState {
        pool,
        upload_dir: Arc::new(upload_dir.clone()),
        signer,
    };

    let cors = CorsLayer::new()
        .allow_origin(frontend_origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .nest("/api/recipes", routes::recipes::router())
        .nest("/api", routes::images::router())
        .nest("/api", routes::ratings::router())
        .nest("/api", routes::share::router())
        // Static file serve for uploaded images. `tower-http::services::ServeDir`
        // streams files (range requests, ETag) without reading them into
        // memory. This is the right primitive for serving user content.
        .nest_service("/uploads", ServeDir::new(&upload_dir))
        .route("/healthz", axum::routing::get(health))
        .with_state(state)
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
