use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::Client;

/// DynamoDB client wrapper for heartbeat monitor storage.
pub struct DynamoStore {
    client: Client,
    table_name: String,
}

impl DynamoStore {
    /// Create a new `DynamoStore` by loading AWS configuration from the
    /// environment and constructing a DynamoDB client.
    pub async fn new(table_name: impl Into<String>) -> Self {
        let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
        let client = Client::new(&config);
        Self {
            client,
            table_name: table_name.into(),
        }
    }

    /// The DynamoDB table name.
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// The underlying DynamoDB client, for direct use in later phases.
    pub fn client(&self) -> &Client {
        &self.client
    }

    // Phase 2: pub async fn upsert_monitor(&self, monitor: &Monitor) -> Result<(), CoreError>
    // Phase 2: pub async fn get_monitor(&self, slug: &Slug) -> Result<Option<Monitor>, CoreError>
    // Phase 3: pub async fn query_overdue(&self, now: i64) -> Result<Vec<Monitor>, CoreError>
    // Phase 4: pub async fn list_monitors(&self) -> Result<Vec<Monitor>, CoreError>
    // Phase 4: pub async fn delete_monitor(&self, slug: &Slug) -> Result<(), CoreError>
}
