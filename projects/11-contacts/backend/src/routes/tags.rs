use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::auth::session::AuthUser;
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct TagWithCount {
    pub name: String,
    pub count: i64,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list))
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<TagWithCount>>> {
    let rows = sqlx::query!(
        r#"
        SELECT t.name,
               COUNT(ct.contact_id) FILTER (WHERE c.deleted_at IS NULL) AS "count!: i64"
        FROM tags t
        LEFT JOIN contact_tags ct ON ct.tag_id = t.id
        LEFT JOIN contacts c ON c.id = ct.contact_id
        WHERE t.user_id = $1
        GROUP BY t.id
        ORDER BY COUNT(ct.contact_id) FILTER (WHERE c.deleted_at IS NULL) DESC, t.name ASC
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
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
