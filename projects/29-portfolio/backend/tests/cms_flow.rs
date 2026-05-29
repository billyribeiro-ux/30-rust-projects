//! Portfolio CMS proofs: post lifecycle (draft → publish → public read)
//! and the markdown sanitiser stripping <script>.

use portfolio_backend::AppState;
use portfolio_backend::build_app;
use portfolio_backend::routes::cms::render_mdx;
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn maybe_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn fresh() -> Option<(SocketAddr, PgPool)> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    sqlx::query!("TRUNCATE posts, sessions, users CASCADE")
        .execute(&pool)
        .await
        .ok()?;
    let state = AppState {
        pool: pool.clone(),
        public_url: "http://localhost:5201".into(),
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5201");
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .ok()?;
    let addr = listener.local_addr().ok()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    Some((addr, pool))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap()
}

#[tokio::test]
async fn draft_is_invisible_until_published() {
    let Some((addr, _)) = fresh().await else {
        eprintln!("skip: no DATABASE_URL");
        return;
    };
    let base = format!("http://{addr}");
    let c = client();
    c.post(format!("{base}/api/auth/register"))
        .json(&serde_json::json!({
            "email": "designer@example.com",
            "password": "correct horse battery",
            "name": "D"
        }))
        .send()
        .await
        .unwrap();
    let r = c
        .post(format!("{base}/api/cms/posts"))
        .json(&serde_json::json!({
            "slug": "first",
            "title": "First",
            "body_mdx": "# Hello\n\nBody."
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let id = r.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Public list — empty.
    let pub_list: Vec<serde_json::Value> = reqwest::get(format!("{base}/api/posts"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(pub_list.is_empty());

    // Public detail — 404.
    let r = reqwest::get(format!("{base}/api/posts/first"))
        .await
        .unwrap();
    assert_eq!(r.status(), 404);

    // Publish.
    let r = c
        .post(format!("{base}/api/cms/posts/{id}/publish"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);

    let pub_list: Vec<serde_json::Value> = reqwest::get(format!("{base}/api/posts"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(pub_list.len(), 1);

    let pub_detail: serde_json::Value = reqwest::get(format!("{base}/api/posts/first"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(pub_detail["body_html"].as_str().unwrap().contains("<h1>"));
}

#[test]
fn markdown_strips_script_tags() {
    let out = render_mdx("# Hi\n<script>alert(1)</script>\n");
    assert!(out.contains("<h1>"));
    assert!(!out.contains("<script>"));
}
