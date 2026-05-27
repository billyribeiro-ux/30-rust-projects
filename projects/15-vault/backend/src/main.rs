mod auth;
mod blob;
mod db;
mod error;
mod routes;
mod state;
mod storage;

use axum::Router;
use axum::http::{HeaderValue, Method};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::state::AppState;
use crate::storage::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vault_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required (see .env.example)");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5187".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3014);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);
    let upload_tmp: PathBuf = std::env::var("VAULT_UPLOAD_TMP")
        .unwrap_or_else(|_| "./data/uploads".into())
        .into();
    std::fs::create_dir_all(&upload_tmp)?;

    let pool = db::connect(&db_url).await?;
    tracing::info!("database ready");
    let storage = Storage::from_env()?;
    tracing::info!("storage ready");

    let state = AppState {
        pool: pool.clone(),
        storage: Arc::new(storage),
        upload_tmp,
        secure_cookies,
    };

    // Reap stale upload sessions every hour. A session older than 24h is
    // assumed abandoned; we delete its row + temp file. The grace window
    // means a user who pauses overnight still resumes successfully.
    let reaper_pool = pool.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            if let Err(e) = routes::uploads::reap_stale(&reaper_pool).await {
                tracing::warn!(error = %e, "stale upload reaper failed");
            }
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(frontend_origin.parse::<HeaderValue>()?)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::HEAD,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderName::from_static("upload-offset"),
        ])
        .expose_headers([axum::http::HeaderName::from_static("upload-offset")])
        .allow_credentials(true);

    let app = Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api/folders", routes::folders::router())
        .nest("/api/files", routes::files::router())
        .nest("/api/uploads", routes::uploads::router())
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
