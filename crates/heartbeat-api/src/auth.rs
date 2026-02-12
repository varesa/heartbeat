use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use aws_sdk_dynamodb::types::AttributeValue;

use crate::state::AppState;

/// An authenticated API key extracted from the `Authorization: Bearer <key>` header.
///
/// Validates the key against the DynamoDB API keys table.
#[allow(dead_code)]
pub struct ApiKey {
    pub key: String,
}

impl FromRequestParts<AppState> for ApiKey {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;

        if token.is_empty() {
            return Err(ApiError::Unauthorized);
        }

        // Look up the key in DynamoDB
        let result = state
            .dynamo_client
            .get_item()
            .table_name(&state.keys_table)
            .key("api_key", AttributeValue::S(token.to_string()))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("DynamoDB key lookup error: {e}");
                ApiError::Internal
            })?;

        if result.item.is_none() {
            return Err(ApiError::Unauthorized);
        }

        Ok(ApiKey {
            key: token.to_string(),
        })
    }
}

/// API error types with JSON responses.
#[derive(Debug)]
pub enum ApiError {
    /// Missing or invalid API key.
    Unauthorized,
    /// Invalid slug format.
    InvalidSlug(String),
    /// Invalid interval value.
    InvalidInterval(String),
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
        tracing::error!("Core error: {err}");
        ApiError::Internal
    }
}
