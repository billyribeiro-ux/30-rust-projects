use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, patch, post};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::{self, AuthUser, SESSION_COOKIE_NAME};
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{slug}", get(detail))
        .route("/{slug}/publish", post(publish))
        .route("/{slug}/lessons", post(add_lesson))
        .route("/{slug}/lessons/{lesson_id}", patch(update_lesson))
}

#[derive(Debug, Serialize)]
pub struct CourseSummary {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub price_cents: i64,
    pub currency: String,
    pub status: String,
    pub instructor_name: String,
    pub created_at: DateTime<Utc>,
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<CourseSummary>>> {
    let rows = sqlx::query!(
        r#"SELECT c.id, c.slug, c.title, c.summary, c.price_cents, c.currency, c.status,
                  u.name AS instructor_name, c.created_at
           FROM courses c JOIN users u ON u.id = c.instructor_id
           WHERE c.status = 'published'
           ORDER BY c.created_at DESC LIMIT 100"#
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| CourseSummary {
                id: r.id,
                slug: r.slug,
                title: r.title,
                summary: r.summary,
                price_cents: r.price_cents,
                currency: r.currency,
                status: r.status,
                instructor_name: r.instructor_name,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct LessonOut {
    pub id: Uuid,
    pub title: String,
    pub position: i32,
    pub duration_seconds: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CourseDetail {
    pub course: CourseSummary,
    pub lessons: Vec<LessonOut>,
    pub enrolled: bool,
}

async fn detail(
    State(s): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> AppResult<Json<CourseDetail>> {
    let row = sqlx::query!(
        r#"SELECT c.id, c.slug, c.title, c.summary, c.price_cents, c.currency, c.status,
                  u.name AS instructor_name, c.created_at
           FROM courses c JOIN users u ON u.id = c.instructor_id
           WHERE c.slug = $1"#,
        slug
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let lessons = sqlx::query!(
        "SELECT id, title, position, duration_seconds
         FROM lessons WHERE course_id = $1 ORDER BY position",
        row.id
    )
    .fetch_all(&s.pool)
    .await?
    .into_iter()
    .map(|r| LessonOut {
        id: r.id,
        title: r.title,
        position: r.position,
        duration_seconds: r.duration_seconds,
    })
    .collect();
    let enrolled = if let Some(raw) = jar.get(SESSION_COOKIE_NAME).map(|c| c.value().to_string()) {
        let token_hash = session::hash_token(&raw);
        sqlx::query_scalar!(
            r#"SELECT 1 AS "x!: i32"
               FROM enrollments e
               JOIN sessions s ON s.token_hash = $2
               WHERE e.course_id = $1 AND e.student_user_id = s.user_id
                 AND e.refunded_at IS NULL AND s.expires_at > now()"#,
            row.id,
            token_hash
        )
        .fetch_optional(&s.pool)
        .await?
        .is_some()
    } else {
        false
    };
    Ok(Json(CourseDetail {
        course: CourseSummary {
            id: row.id,
            slug: row.slug,
            title: row.title,
            summary: row.summary,
            price_cents: row.price_cents,
            currency: row.currency,
            status: row.status,
            instructor_name: row.instructor_name,
            created_at: row.created_at,
        },
        lessons,
        enrolled,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateCourseInput {
    pub slug: String,
    pub title: String,
    pub summary: Option<String>,
    pub price_cents: i64,
    pub currency: Option<String>,
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(i): Json<CreateCourseInput>,
) -> AppResult<impl IntoResponse> {
    if i.price_cents <= 0 {
        return Err(AppError::Fields(vec![FieldError {
            field: "price_cents".into(),
            message: "must be > 0".into(),
        }]));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO courses (id, instructor_id, slug, title, summary, price_cents, currency)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        id,
        user.id,
        i.slug,
        i.title,
        i.summary.unwrap_or_default(),
        i.price_cents,
        i.currency.unwrap_or_else(|| "usd".into())
    )
    .execute(&s.pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("courses_slug_key") => {
            AppError::Conflict("slug in use".into())
        }
        _ => e.into(),
    })?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

async fn publish(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    let n = sqlx::query!(
        "UPDATE courses SET status = 'published', updated_at = now()
         WHERE slug = $1 AND instructor_id = $2",
        slug,
        user.id
    )
    .execute(&s.pool)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct AddLessonInput {
    pub title: String,
    pub position: i32,
    pub duration_seconds: Option<i32>,
}

async fn add_lesson(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
    Json(i): Json<AddLessonInput>,
) -> AppResult<impl IntoResponse> {
    let course = sqlx::query!(
        "SELECT id FROM courses WHERE slug = $1 AND instructor_id = $2",
        slug,
        user.id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO lessons (id, course_id, title, position, duration_seconds)
         VALUES ($1, $2, $3, $4, $5)",
        id,
        course.id,
        i.title,
        i.position,
        i.duration_seconds
    )
    .execute(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

#[derive(Debug, Deserialize)]
pub struct UpdateLessonInput {
    pub title: Option<String>,
    pub position: Option<i32>,
    pub duration_seconds: Option<i32>,
    pub hls_manifest: Option<String>,
}

async fn update_lesson(
    State(s): State<AppState>,
    user: AuthUser,
    Path((slug, lesson_id)): Path<(String, Uuid)>,
    Json(i): Json<UpdateLessonInput>,
) -> AppResult<impl IntoResponse> {
    let n = sqlx::query!(
        r#"UPDATE lessons l
           SET title = COALESCE($3, l.title),
               position = COALESCE($4, l.position),
               duration_seconds = COALESCE($5, l.duration_seconds),
               hls_manifest = COALESCE($6, l.hls_manifest)
           FROM courses c
           WHERE l.id = $2 AND c.id = l.course_id AND c.slug = $1
             AND c.instructor_id = $7"#,
        slug,
        lesson_id,
        i.title,
        i.position,
        i.duration_seconds,
        i.hls_manifest,
        user.id
    )
    .execute(&s.pool)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
