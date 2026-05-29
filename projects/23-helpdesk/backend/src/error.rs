use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation: {0}")]
    Validation(String),
    #[error("validation (fields)")]
    Fields(Vec<FieldError>),
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal: {0}")]
    Internal(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Serialize)]
struct Body<'a> {
    error: ErrBody<'a>,
}
#[derive(Serialize)]
struct ErrBody<'a> {
    code: &'a str,
    message: String,
    fields: Option<&'a [FieldError]>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, msg, fields): (StatusCode, &str, String, Option<Vec<FieldError>>) =
            match self {
                AppError::Validation(m) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, "validation", m, None)
                }
                AppError::Fields(f) => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation",
                    "validation failed".into(),
                    Some(f),
                ),
                AppError::NotFound => {
                    (StatusCode::NOT_FOUND, "not_found", "not found".into(), None)
                }
                AppError::Unauthorized => (
                    StatusCode::UNAUTHORIZED,
                    "unauthorized",
                    "unauthorized".into(),
                    None,
                ),
                AppError::Forbidden => {
                    (StatusCode::FORBIDDEN, "forbidden", "forbidden".into(), None)
                }
                AppError::Conflict(m) => (StatusCode::CONFLICT, "conflict", m, None),
                AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", m, None),
                AppError::Database(e) => {
                    tracing::error!(?e, "database error");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal",
                        "internal error".into(),
                        None,
                    )
                }
            };
        let body = Body {
            error: ErrBody {
                code,
                message: msg,
                fields: fields.as_deref(),
            },
        };
        (status, Json(body)).into_response()
    }
}
