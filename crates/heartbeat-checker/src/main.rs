mod alerts;
mod checker;
mod telegram;

use std::env;

use aws_config::BehaviorVersion;
use heartbeat_core::DynamoStore;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::info;
use tracing_subscriber::EnvFilter;

use telegram::TelegramClient;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize structured JSON logging for CloudWatch
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    // Read table name from environment
    let table_name =
        env::var("HEARTBEAT_TABLE_NAME").unwrap_or_else(|_| "heartbeat-monitors".to_string());

    info!(table_name = %table_name, "initializing heartbeat checker");

    // Create DynamoDB store
    let store = DynamoStore::new(&table_name).await;

    // Read Telegram credentials from SSM Parameter Store
    let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let ssm = aws_sdk_ssm::Client::new(&config);

    let bot_token_param = env::var("TELEGRAM_BOT_TOKEN_PARAM")
        .unwrap_or_else(|_| "/heartbeat/telegram-bot-token".to_string());
    let chat_id_param = env::var("TELEGRAM_CHAT_ID_PARAM")
        .unwrap_or_else(|_| "/heartbeat/telegram-chat-id".to_string());

    let bot_token = ssm
        .get_parameter()
        .name(&bot_token_param)
        .with_decryption(true)
        .send()
        .await?
        .parameter()
        .and_then(|p| p.value().map(String::from))
        .ok_or_else(|| Error::from("missing SSM parameter for bot token"))?;

    let chat_id = ssm
        .get_parameter()
        .name(&chat_id_param)
        .with_decryption(true)
        .send()
        .await?
        .parameter()
        .and_then(|p| p.value().map(String::from))
        .ok_or_else(|| Error::from("missing SSM parameter for chat id"))?;

    let telegram = TelegramClient::new(bot_token, chat_id);

    info!("cold start complete, starting Lambda runtime");

    // Run the Lambda runtime
    lambda_runtime::run(service_fn(|_event: LambdaEvent<serde_json::Value>| {
        let store = store.clone();
        let telegram = telegram.clone();
        async move {
            checker::check_monitors(&store, &telegram)
                .await
                .map_err(|e| Error::from(e.to_string()))?;
            Ok::<serde_json::Value, Error>(serde_json::json!({"status": "ok"}))
        }
    }))
    .await
}
