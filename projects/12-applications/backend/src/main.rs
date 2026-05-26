mod auth;
mod db;
mod email;
mod error;
mod reminders;
mod routes;
mod state;

use axum::Router;
use axum::http::{HeaderValue, Method};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::email::Mailer;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "applications_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required (see .env.example)");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5184".to_string());
    let smtp_url =
        std::env::var("SMTP_URL").unwrap_or_else(|_| "smtp://localhost:1025".to_string());
    let smtp_from =
        std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@applications.local".to_string());
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:5184".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3011);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);

    let pool = db::connect(&db_url).await?;
    tracing::info!("database ready");

    let mailer = Mailer::new(&smtp_url, &smtp_from)?;
    let state = AppState {
        pool,
        mailer: Arc::new(mailer),
        public_url,
        secure_cookies,
    };

    let cors = CorsLayer::new()
        .allow_origin(frontend_origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    // Spawn the background reminder task. Cheap clone (AppState fields are
    // Arc-backed). The task lives for the process lifetime; on shutdown the
    // tokio runtime drops it.
    let reminder_state = state.clone();
    let reminder_interval = std::env::var("REMINDER_TICK_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60); // 60s in dev, override to 3600 in prod
    tokio::spawn(async move {
        reminders::run(
            reminder_state,
            std::time::Duration::from_secs(reminder_interval),
        )
        .await;
    });
    tracing::info!(tick_secs = reminder_interval, "reminder task spawned");

    let app = Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api/applications", routes::applications::router())
        .nest(
            "/api/applications/{application_id}/events",
            routes::events::router(),
        )
        .nest(
            "/api/applications/{application_id}/next-steps",
            routes::next_steps::router(),
        )
        .nest("/api/dashboard", routes::dashboard::router())
        .nest("/api/export", routes::export::router())
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
