//! Instructor onboarding: become-an-instructor + start-stripe-onboarding.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/me/onboard", post(onboard))
}

#[derive(Debug, Serialize)]
pub struct InstructorOut {
    pub user_id: uuid::Uuid,
    pub stripe_account_id: Option<String>,
    pub payouts_enabled: bool,
    pub details_submitted: bool,
    pub created_at: DateTime<Utc>,
}

async fn me(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<InstructorOut>> {
    let row = sqlx::query!(
        "SELECT user_id, stripe_account_id, payouts_enabled, details_submitted, created_at
         FROM instructors WHERE user_id = $1",
        user.id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(InstructorOut {
        user_id: row.user_id,
        stripe_account_id: row.stripe_account_id,
        payouts_enabled: row.payouts_enabled,
        details_submitted: row.details_submitted,
        created_at: row.created_at,
    }))
}

#[derive(Debug, Serialize)]
pub struct OnboardOut {
    pub onboarding_url: String,
}

async fn onboard(State(s): State<AppState>, user: AuthUser) -> AppResult<impl IntoResponse> {
    // Ensure an instructors row exists.
    let row = sqlx::query!(
        "INSERT INTO instructors (user_id) VALUES ($1)
         ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
         RETURNING stripe_account_id",
        user.id
    )
    .fetch_one(&s.pool)
    .await?;

    let account_id = match row.stripe_account_id {
        Some(id) => id,
        None => {
            let acc = s.stripe.create_express_account(&user.email, "US").await?;
            sqlx::query!(
                "UPDATE instructors SET stripe_account_id = $2, updated_at = now()
                 WHERE user_id = $1",
                user.id,
                acc.id
            )
            .execute(&s.pool)
            .await?;
            acc.id
        }
    };

    let return_url = format!("{}/instructor?return=1", s.public_url);
    let refresh_url = format!("{}/instructor?refresh=1", s.public_url);
    let link = s
        .stripe
        .create_account_link(&account_id, &return_url, &refresh_url)
        .await?;

    // Mark user as instructor.
    sqlx::query!(
        "UPDATE users SET role = 'instructor' WHERE id = $1",
        user.id
    )
    .execute(&s.pool)
    .await?;
    Ok((
        StatusCode::OK,
        Json(OnboardOut {
            onboarding_url: link.url,
        }),
    ))
}
