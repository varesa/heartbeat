use aws_sdk_dynamodb::Client;
use heartbeat_core::DynamoStore;

/// Shared application state passed to all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    /// DynamoDB store for monitor operations.
    pub monitors_store: DynamoStore,
    /// DynamoDB client for API key lookups.
    pub dynamo_client: Client,
    /// DynamoDB table name for API keys.
    pub keys_table: String,
}
