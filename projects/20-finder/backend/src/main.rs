use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use finder_backend::oauth::provider::{GithubProvider, GoogleProvider, OAuthProvider, ProviderId};
use finder_backend::oauth::{OAuthRegistry, ProviderConfig};
use finder_backend::state::AppState;
use finder_backend::{build_app, db};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "finder_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required (see .env.example)");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5192".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3019);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);
    let redirect_base =
        std::env::var("OAUTH_REDIRECT_BASE").unwrap_or_else(|_| format!("http://localhost:{port}"));
    let success =
        std::env::var("OAUTH_SUCCESS_REDIRECT").unwrap_or_else(|_| format!("{frontend_origin}/"));
    let failure = std::env::var("OAUTH_FAILURE_REDIRECT")
        .unwrap_or_else(|_| format!("{frontend_origin}/login?error=oauth"));

    let pool = db::connect(&db_url).await?;
    tracing::info!("database ready");

    let mut registry = OAuthRegistry::new();
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("reqwest client builds");

    if let (Ok(client_id), Ok(client_secret)) = (
        std::env::var("GOOGLE_OAUTH_CLIENT_ID"),
        std::env::var("GOOGLE_OAUTH_CLIENT_SECRET"),
    ) && !client_id.is_empty()
        && !client_secret.is_empty()
    {
        let cfg = ProviderConfig {
            client_id,
            client_secret,
            authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo".into(),
            redirect_uri: format!("{redirect_base}/api/auth/oauth/google/callback"),
            scope: "openid email profile",
        };
        registry.insert(Box::new(GoogleProvider::new(cfg, http.clone())) as Box<dyn OAuthProvider>);
        tracing::info!("oauth provider enabled: google");
    }
    if let (Ok(client_id), Ok(client_secret)) = (
        std::env::var("GITHUB_OAUTH_CLIENT_ID"),
        std::env::var("GITHUB_OAUTH_CLIENT_SECRET"),
    ) && !client_id.is_empty()
        && !client_secret.is_empty()
    {
        let cfg = ProviderConfig {
            client_id,
            client_secret,
            authorize_url: "https://github.com/login/oauth/authorize".into(),
            token_url: "https://github.com/login/oauth/access_token".into(),
            userinfo_url: "https://api.github.com/user".into(),
            redirect_uri: format!("{redirect_base}/api/auth/oauth/github/callback"),
            scope: "read:user user:email",
        };
        registry.insert(Box::new(GithubProvider::new(cfg, http.clone())) as Box<dyn OAuthProvider>);
        tracing::info!("oauth provider enabled: github");
    }
    if registry.is_empty() {
        tracing::warn!("no OAuth provider credentials configured — only password auth available");
    }
    let _ = ProviderId::Google;

    let state = AppState {
        pool,
        oauth: Arc::new(registry),
        secure_cookies,
        oauth_success_redirect: success,
        oauth_failure_redirect: failure,
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
