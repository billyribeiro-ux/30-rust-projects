use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::AppResult;

#[derive(Debug, Serialize)]
pub struct TagWithCount {
    pub name: String,
    pub count: i64,
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/", get(list))
}

async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<TagWithCount>>> {
    let rows = sqlx::query!(
        r#"
        SELECT t.name, CAST(COUNT(bt.bookmark_id) AS INTEGER) AS "count!: i64"
        FROM tags t
        LEFT JOIN bookmark_tags bt ON bt.tag_id = t.id
        GROUP BY t.id
        ORDER BY COUNT(bt.bookmark_id) DESC, t.name ASC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| TagWithCount {
                name: r.name,
                count: r.count,
            })
            .collect(),
    ))
}
