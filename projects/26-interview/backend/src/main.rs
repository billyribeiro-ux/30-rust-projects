use interview_backend::AppState;
use interview_backend::{build_app, db, make_hubs};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "interview_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5198".into());
    let public_url = std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:5198".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3025);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);
    let saml_entity_id = std::env::var("SAML_ENTITY_ID")
        .unwrap_or_else(|_| "http://localhost:3025/saml/metadata".into());
    let saml_acs_url =
        std::env::var("SAML_ACS_URL").unwrap_or_else(|_| "http://localhost:3025/saml/acs".into());

    let pool = db::connect(&db_url).await?;
    let state = AppState {
        pool,
        hubs: make_hubs(),
        public_url,
        saml_entity_id,
        saml_acs_url,
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
