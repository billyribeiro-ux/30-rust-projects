use newsletter_backend::state::AppState;
use newsletter_backend::stripe::client::StripeClient;
use newsletter_backend::{build_app, db};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "newsletter_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required (see .env.example)");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5193".to_string());
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:5193".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3020);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);

    let stripe_secret =
        std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_local".into());
    let stripe_webhook_secret =
        std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_else(|_| "whsec_local".into());
    let stripe_price_id = std::env::var("STRIPE_PRICE_ID").unwrap_or_else(|_| "price_local".into());
    let smtp_url = std::env::var("SMTP_URL").unwrap_or_else(|_| "smtp://localhost:1026".into());
    let smtp_from =
        std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@newsletter.local".into());

    let pool = db::connect(&db_url).await?;
    tracing::info!("database ready");

    let stripe = Arc::new(StripeClient::new(stripe_secret));

    let state = AppState {
        pool,
        stripe,
        stripe_webhook_secret,
        stripe_price_id,
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
