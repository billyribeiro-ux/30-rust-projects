use axum::Router;
use axum::http::{HeaderValue, Method};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use storefront_backend::db;
use storefront_backend::email::Mailer;
use storefront_backend::routes;
use storefront_backend::state::AppState;
use storefront_backend::stripe::client::StripeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "storefront_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required (see .env.example)");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5188".to_string());
    let smtp_url =
        std::env::var("SMTP_URL").unwrap_or_else(|_| "smtp://localhost:1025".to_string());
    let smtp_from =
        std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@storefront.local".to_string());
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:5188".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3015);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);

    let stripe_api_base =
        std::env::var("STRIPE_API_BASE").unwrap_or_else(|_| "https://api.stripe.com".to_string());
    let stripe_secret_key =
        std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_dummy".to_string());
    let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .unwrap_or_else(|_| "whsec_dummy".to_string())
        .into_bytes();
    let download_signing_secret = std::env::var("DOWNLOAD_SIGNING_SECRET")
        .unwrap_or_else(|_| {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        })
        .into_bytes();
    let product_files_dir = PathBuf::from(
        std::env::var("PRODUCT_FILES_DIR").unwrap_or_else(|_| "./data/products".to_string()),
    );

    let pool = db::connect(&db_url).await?;
    tracing::info!("database ready");

    tokio::fs::create_dir_all(&product_files_dir).await?;

    let mailer = Mailer::new(&smtp_url, &smtp_from)?;
    let stripe_client = StripeClient::new(&stripe_api_base, &stripe_secret_key)?;
    let state = AppState {
        pool,
        mailer: Arc::new(mailer),
        stripe: Arc::new(stripe_client),
        public_url,
        secure_cookies,
        download_signing_secret: Arc::new(download_signing_secret),
        stripe_webhook_secret: Arc::new(stripe_webhook_secret),
        product_files_dir,
    };

    let cors = CorsLayer::new()
        .allow_origin(frontend_origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api", routes::public::router())
        .nest("/api/stripe", routes::stripe::router())
        .nest("/api/admin", routes::admin::router())
        .nest("/d", routes::download::router())
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
