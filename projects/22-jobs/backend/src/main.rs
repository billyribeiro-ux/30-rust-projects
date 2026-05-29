use jobs_backend::jobs::Registry;
use jobs_backend::jobs::registry::HandlerFn;
use jobs_backend::jobs::runner::{Config, run_worker};
use jobs_backend::state::AppState;
use jobs_backend::{build_app, db};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jobs_backend=info,tower_http=info".into()),
        )
        .with_target(false)
        .compact()
        .init();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required (see .env.example)");
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5194".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3021);
    let secure_cookies = std::env::var("SECURE_COOKIES")
        .map(|v| v == "true")
        .unwrap_or(false);
    let run_workers = std::env::var("RUN_WORKERS")
        .map(|v| v == "1")
        .unwrap_or(true);
    let worker_concurrency: usize = std::env::var("WORKER_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let lease_secs: i64 = std::env::var("WORKER_LEASE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let pool = db::connect(&db_url).await?;
    tracing::info!("database ready");

    // Built-in handlers — kept in main so the project ships with demo jobs.
    let mut registry = Registry::new();
    let send_email: HandlerFn = Arc::new(|payload| {
        Box::pin(async move {
            let to = payload.get("to").and_then(|v| v.as_str()).unwrap_or("?");
            tracing::info!(%to, "send_email noop");
            Ok::<String, jobs_backend::error::AppError>(format!("noop send to {to}"))
        })
    });
    let resize_image: HandlerFn = Arc::new(|payload| {
        Box::pin(async move {
            let url = payload.get("url").and_then(|v| v.as_str()).unwrap_or("?");
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            Ok::<String, jobs_backend::error::AppError>(format!("resized {url}"))
        })
    });
    let flaky_job: HandlerFn = Arc::new(|payload| {
        Box::pin(async move {
            use rand::Rng;
            let fail_pct = payload
                .get("fail_pct")
                .and_then(|v| v.as_u64())
                .unwrap_or(40);
            if rand::thread_rng().gen_range(0..100) < fail_pct {
                return Err(jobs_backend::error::AppError::Internal(
                    "flaky job rolled a failure".into(),
                ));
            }
            Ok::<String, jobs_backend::error::AppError>("flaky succeeded".into())
        })
    });
    registry.register("send_email", send_email);
    registry.register("resize_image", resize_image);
    registry.register("flaky_job", flaky_job);
    let registry = Arc::new(registry);

    let (events_tx, _) = broadcast::channel(1024);

    let state = AppState {
        pool: pool.clone(),
        registry: registry.clone(),
        events: events_tx.clone(),
        secure_cookies,
    };

    if run_workers {
        let cfg = Config {
            concurrency: worker_concurrency,
            lease_secs,
            poll_idle_ms: 1000,
            batch_size: 8,
        };
        let r = registry.clone();
        let p = pool.clone();
        let e = events_tx.clone();
        tokio::spawn(async move {
            run_worker(p, r, e, "default".into(), cfg).await;
        });
        tracing::info!(
            concurrency = worker_concurrency,
            "worker spawned for 'default' queue"
        );
    } else {
        tracing::info!("RUN_WORKERS=0 — API-only mode");
    }

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
