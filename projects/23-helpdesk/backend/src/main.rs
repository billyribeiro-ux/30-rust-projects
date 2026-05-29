use helpdesk_backend::state::AppState;
use helpdesk_backend::{build_app, db};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "helpdesk_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required (see .env.example)");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5195".to_string());
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:5195".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3022);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);
    let smtp_url = std::env::var("SMTP_URL").unwrap_or_else(|_| "smtp://localhost:1027".into());
    let smtp_from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@helpdesk.local".into());

    let pool = db::connect(&db_url).await?;
    let state = AppState {
        pool,
        public_url,
        smtp_url,
        smtp_from,
        secure_cookies,
    };
    let app = build_app(state, &frontend_origin);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
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
