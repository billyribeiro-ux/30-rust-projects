use marketplace_backend::AppState;
use marketplace_backend::stripe::client::StripeClient;
use marketplace_backend::{build_app, db};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "marketplace_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5197".into());
    let public_url = std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:5197".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3024);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);
    let stripe_secret =
        std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_local".into());
    let stripe_webhook_secret =
        std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_else(|_| "whsec_local".into());
    let platform_fee_bps: i64 = std::env::var("PLATFORM_FEE_BPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let download_signing_key = std::env::var("DOWNLOAD_SIGNING_KEY").unwrap_or_else(|_| {
        "0000000000000000000000000000000000000000000000000000000000000000".into()
    });

    let pool = db::connect(&db_url).await?;
    let stripe = Arc::new(StripeClient::new(stripe_secret));
    let state = AppState {
        pool,
        stripe,
        stripe_webhook_secret,
        platform_fee_bps,
        public_url,
        download_signing_key,
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
        tokio::signal::ctrl_c().await.unwrap();
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .unwrap()
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}
