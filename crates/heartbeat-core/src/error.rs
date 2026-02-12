use thiserror::Error;

/// Core errors for the heartbeat system.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("DynamoDB error: {0}")]
    Dynamo(#[from] aws_sdk_dynamodb::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_dynamo::Error),

    #[error("Slug validation error: {0}")]
    Slug(#[from] crate::model::SlugError),
}
