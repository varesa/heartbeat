use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use aws_sdk_dynamodb::types::AttributeValue;

use crate::errors::ApiError;
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
