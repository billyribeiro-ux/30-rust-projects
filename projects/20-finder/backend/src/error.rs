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

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    #[allow(dead_code)]
    Forbidden,

    #[error("conflict: {0}")]
    Conflict(String),

    /// Upstream (OAuth provider) returned an error or unexpected payload.
    #[error("upstream: {0}")]
    Upstream(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("internal: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Upstream(e.to_string())
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
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", None),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", None),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict", None),
            AppError::Upstream(msg) => {
                tracing::warn!(error = %msg, "upstream error");
                (StatusCode::BAD_GATEWAY, "upstream_error", None)
            }
            AppError::Database(err) => {
                tracing::error!(error = ?err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", None)
            }
            AppError::Internal(msg) => {
                tracing::error!(error = %msg, "internal error");
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
