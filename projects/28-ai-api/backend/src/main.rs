use ai_api_backend::AppState;
use ai_api_backend::{build_app, build_webauthn, db};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ai_api_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5200".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3027);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);
    let rp_id = std::env::var("WEBAUTHN_RP_ID").unwrap_or_else(|_| "localhost".into());
    let rp_name = std::env::var("WEBAUTHN_RP_NAME").unwrap_or_else(|_| "AI Inference API".into());
    let webauthn_origin =
        std::env::var("WEBAUTHN_ORIGIN").unwrap_or_else(|_| "http://localhost:5200".into());
    let free_tier_calls: i64 = std::env::var("FREE_TIER_CALLS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000);
    let per_call_cents: f64 = std::env::var("PER_CALL_CENTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.1);

    let pool = db::connect(&db_url).await?;
    let webauthn = build_webauthn(&rp_id, &rp_name, &webauthn_origin);
    let state = AppState {
        pool,
        webauthn,
        free_tier_calls,
        per_call_cents,
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
