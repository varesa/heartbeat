use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;

use crate::error::CoreError;
use crate::model::{Monitor, Slug};

/// DynamoDB client wrapper for heartbeat monitor storage.
#[derive(Clone)]
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

    /// Upsert a monitor into DynamoDB using `update_item`.
    ///
    /// Uses `if_not_exists` for `created_at` so the original creation
    /// timestamp is preserved on subsequent pings.
    pub async fn upsert_monitor(&self, monitor: &Monitor) -> Result<(), CoreError> {
        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("slug", AttributeValue::S(monitor.slug.clone()))
            .update_expression(
                "SET interval_secs = :interval, \
                 last_ping = :last_ping, \
                 next_due = :next_due, \
                 check_partition = :cp, \
                 expires_at = :expires, \
                 created_at = if_not_exists(created_at, :created_at)",
            )
            .expression_attribute_values(
                ":interval",
                AttributeValue::N(monitor.interval_secs.to_string()),
            )
            .expression_attribute_values(
                ":last_ping",
                AttributeValue::N(monitor.last_ping.to_string()),
            )
            .expression_attribute_values(
                ":next_due",
                AttributeValue::N(monitor.next_due.to_string()),
            )
            .expression_attribute_values(
                ":cp",
                AttributeValue::S(monitor.check_partition.clone()),
            )
            .expression_attribute_values(
                ":expires",
                AttributeValue::N(monitor.expires_at.to_string()),
            )
            .expression_attribute_values(
                ":created_at",
                AttributeValue::N(monitor.created_at.to_string()),
            )
            .send()
            .await
            .map_err(|e| CoreError::DynamoSdk(Box::new(e)))?;

        Ok(())
    }

    /// Get a monitor by slug.
    ///
    /// Returns `None` if the monitor does not exist.
    pub async fn get_monitor(&self, slug: &Slug) -> Result<Option<Monitor>, CoreError> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("slug", AttributeValue::S(slug.to_string()))
            .send()
            .await
            .map_err(|e| CoreError::DynamoSdk(Box::new(e)))?;

        match result.item {
            Some(item) => {
                let monitor: Monitor = serde_dynamo::from_item(item)?;
                Ok(Some(monitor))
            }
            None => Ok(None),
        }
    }

    // Phase 3: pub async fn query_overdue(&self, now: i64) -> Result<Vec<Monitor>, CoreError>
    // Phase 4: pub async fn list_monitors(&self) -> Result<Vec<Monitor>, CoreError>
    // Phase 4: pub async fn delete_monitor(&self, slug: &Slug) -> Result<(), CoreError>
}
