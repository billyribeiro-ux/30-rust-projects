mod auth;
mod db;
mod email;
mod error;
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
                .unwrap_or_else(|_| "shortener_backend=info,tower_http=info".into()),
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
        std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@shortener.local".to_string());
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:5188".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3015);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);

    let pool = db::connect(&db_url).await?;
    tracing::info!("database ready");

    // Redis is OPTIONAL. We try to build a pool and ping it once; on any
    // failure (env unset, connection refused, etc.) we run without it and
    // the redirect path falls back to "just write the click row".
    let redis = match std::env::var("REDIS_URL").ok() {
        Some(url) if !url.is_empty() => match build_redis_pool(&url).await {
            Ok(pool) => {
                tracing::info!("redis ready at {url}");
                Some(pool)
            }
            Err(e) => {
                tracing::warn!(error = %e, "redis unavailable; running in DB-only mode");
                None
            }
        },
        _ => {
            tracing::info!("REDIS_URL unset; running in DB-only mode (counter cache disabled)");
            None
        }
    };

    let mailer = Mailer::new(&smtp_url, &smtp_from)?;
    let state = AppState {
        pool,
        redis,
        mailer: Arc::new(mailer),
        public_url,
        secure_cookies,
    };

    let cors = CorsLayer::new()
        .allow_origin(frontend_origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api/links", routes::links::router())
        .nest("/api/2fa", routes::twofa::router())
        .route("/healthz", axum::routing::get(health))
        // IMPORTANT: top-level slug route. This is the hot path — no auth,
        // no SSR, just DB + Redis + spawn(insert). MUST be registered AFTER
        // the /api nests so axum routes /api/* there instead of treating
        // "api" as a slug. Also after /healthz for the same reason.
        .route(
            "/{slug}",
            axum::routing::get(routes::redirect::redirect_to_target),
        )
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

async fn build_redis_pool(url: &str) -> Result<deadpool_redis::Pool, Box<dyn std::error::Error>> {
    let cfg = deadpool_redis::Config::from_url(url);
    let pool = cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
    // Ping once to verify connectivity at startup.
    let mut conn = pool.get().await?;
    let _: String = deadpool_redis::redis::cmd("PING")
        .query_async(&mut conn)
        .await?;
    Ok(pool)
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
