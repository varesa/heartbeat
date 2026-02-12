use thiserror::Error;

/// Core errors for the heartbeat system.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("DynamoDB error: {0}")]
    Dynamo(#[from] aws_sdk_dynamodb::Error),

    #[error("DynamoDB SDK error: {0}")]
    DynamoSdk(Box<dyn std::error::Error + Send + Sync>),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_dynamo::Error),

    #[error("Slug validation error: {0}")]
    Slug(#[from] crate::model::SlugError),

    #[error("Not found: {0}")]
    NotFound(String),
}
