use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;

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

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("payload too large: {0}")]
    TooLarge(String),

    #[error("unsupported media type: {0}")]
    Unsupported(String),

    #[error("internal io: {0}")]
    Io(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<axum::extract::multipart::MultipartError> for AppError {
    fn from(e: axum::extract::multipart::MultipartError) -> Self {
        AppError::Validation(format!("multipart parse error: {e}"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, fields) = match &self {
            AppError::Validation(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed", None)
            }
            AppError::Fields(errs) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                Some(errs.clone()),
            ),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found", None),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict", None),
            AppError::TooLarge(_) => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", None),
            AppError::Unsupported(_) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                None,
            ),
            AppError::Io(err) => {
                tracing::error!(error = %err, "io error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", None)
            }
            AppError::Database(err) => {
                tracing::error!(error = ?err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", None)
            }
        };

        let body = Json(json!({
            "error": { "code": code, "message": self.to_string(), "fields": fields }
        }));
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
