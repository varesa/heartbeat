use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// API error types with JSON responses.
#[derive(Debug)]
pub enum ApiError {
    /// Missing or invalid API key.
    Unauthorized,
    /// Invalid slug format.
    InvalidSlug(String),
    /// Invalid interval value.
    InvalidInterval(String),
    /// Resource not found.
    NotFound(String),
    /// Internal server error.
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Invalid or missing API key".to_string(),
            ),
            ApiError::InvalidSlug(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InvalidInterval(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = serde_json::json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}

impl From<heartbeat_core::CoreError> for ApiError {
    fn from(err: heartbeat_core::CoreError) -> Self {
        match err {
            heartbeat_core::CoreError::NotFound(msg) => ApiError::NotFound(msg),
            other => {
                tracing::error!("Core error: {other}");
                ApiError::Internal
            }
        }
    }
}
