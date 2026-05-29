//! Student-facing purchase endpoint. Creates a Stripe destination
//! charge with our `platform_fee_bps` cut and the instructor's
//! Connect account as the destination.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/checkout/{slug}", post(checkout))
        .route("/refund/{course_id}", post(refund))
}

#[derive(Debug, Serialize)]
pub struct CheckoutOut {
    pub checkout_url: String,
}

async fn checkout(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    let course = sqlx::query!(
        r#"SELECT c.id, c.title, c.price_cents, c.currency,
                  i.stripe_account_id, i.payouts_enabled
           FROM courses c JOIN instructors i ON i.user_id = c.instructor_id
           WHERE c.slug = $1 AND c.status = 'published'"#,
        slug
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let dest = course
        .stripe_account_id
        .ok_or_else(|| AppError::Validation("instructor not onboarded".into()))?;
    if !course.payouts_enabled {
        return Err(AppError::Validation(
            "instructor payouts not enabled yet".into(),
        ));
    }

    let fee = course.price_cents * s.platform_fee_bps / 10_000;
    let success = format!("{}/c/{}?ok=1", s.public_url, slug);
    let cancel = format!("{}/c/{}?cancelled=1", s.public_url, slug);
    let session = s
        .stripe
        .create_destination_checkout(
            &user.email,
            &course.title,
            course.price_cents,
            &course.currency,
            fee,
            &dest,
            &success,
            &cancel,
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(CheckoutOut {
            checkout_url: session.url,
        }),
    ))
}

async fn refund(
    State(s): State<AppState>,
    user: AuthUser,
    Path(course_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    // 14-day refund window. Anything older needs admin override.
    let row = sqlx::query!(
        r#"SELECT stripe_payment_intent_id, created_at, refunded_at
           FROM enrollments
           WHERE course_id = $1 AND student_user_id = $2"#,
        course_id,
        user.id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    if row.refunded_at.is_some() {
        return Err(AppError::Conflict("already refunded".into()));
    }
    let age = chrono::Utc::now() - row.created_at;
    if age > chrono::Duration::days(14) {
        return Err(AppError::Forbidden);
    }
    s.stripe
        .refund_payment_intent(&row.stripe_payment_intent_id)
        .await?;
    sqlx::query!(
        "UPDATE enrollments SET refunded_at = now()
         WHERE course_id = $1 AND student_user_id = $2",
        course_id,
        user.id
    )
    .execute(&s.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
