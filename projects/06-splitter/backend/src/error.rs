use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;

/// A field-level validation error: `{ field: "amount_cents", message: "..." }`.
/// Returned as a JSON array under `error.fields` so the client can map errors
/// to specific form inputs.
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

    #[error(transparent)]
    Database(#[from] sqlx::Error),
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
            AppError::Database(err) => {
                tracing::error!(error = ?err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", None)
            }
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": self.to_string(),
                "fields": fields,
            }
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
