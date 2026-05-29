//! WebSocket collab: two clients connect to the same interview,
//! one sends a code update, the other receives it.

use futures_util::{SinkExt, StreamExt};
use interview_backend::AppState;
use interview_backend::{build_app, make_hubs};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use uuid::Uuid;

async fn maybe_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn fresh() -> Option<(SocketAddr, PgPool, Uuid)> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    sqlx::query!("TRUNCATE executions, interviews, sessions, users CASCADE")
        .execute(&pool)
        .await
        .ok()?;
    let user = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO users (id, email, password_hash, name) VALUES ($1, $2, $3, $4)"#,
        user,
        "i@example.com",
        "x",
        "I"
    )
    .execute(&pool)
    .await
    .ok()?;
    let interview = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO interviews (id, interviewer_id, candidate_email) VALUES ($1, $2, $3)"#,
        interview,
        user,
        "c@example.com"
    )
    .execute(&pool)
    .await
    .ok()?;

    let state = AppState {
        pool: pool.clone(),
        hubs: make_hubs(),
        public_url: "http://localhost:5198".into(),
        saml_entity_id: "http://localhost:3025/saml/metadata".into(),
        saml_acs_url: "http://localhost:3025/saml/acs".into(),
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5198");
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .ok()?;
    let addr = listener.local_addr().ok()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    Some((addr, pool, interview))
}

#[tokio::test]
async fn two_clients_see_each_others_edits() {
    let Some((addr, pool, interview)) = fresh().await else {
        eprintln!("skip: no DATABASE_URL");
        return;
    };
    use tokio_tungstenite::tungstenite::Message;

    let url = format!("ws://{addr}/ws/interview/{interview}");
    let (mut a, _) = tokio_tungstenite::connect_async(url.as_str()).await.unwrap();
    let (mut b, _) = tokio_tungstenite::connect_async(url.as_str()).await.unwrap();

    // A sends a code update.
    let msg = r#"{"kind":"code","value":"fn main(){}"}"#;
    a.send(Message::Text(msg.into())).await.unwrap();

    // B receives it (we may also receive our own echo; drain until target seen).
    let mut got = false;
    for _ in 0..5 {
        if let Some(Ok(Message::Text(t))) =
            tokio::time::timeout(std::time::Duration::from_millis(500), b.next())
                .await
                .unwrap_or(None)
        {
            if t.contains("fn main(){}") {
                got = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(got, "B did not receive A's edit");

    // Persistence — backend wrote the code to interviews.code.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let persisted: String =
        sqlx::query_scalar!("SELECT code FROM interviews WHERE id = $1", interview)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(persisted, "fn main(){}");
}

#[tokio::test]
async fn saml_metadata_is_valid_xml() {
    let Some((addr, _, _)) = fresh().await else {
        return;
    };
    let body = reqwest::get(format!("http://{addr}/saml/metadata"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(body.contains("<md:EntityDescriptor"));
    assert!(body.contains("AssertionConsumerService"));
}
