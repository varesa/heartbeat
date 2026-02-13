use std::time::{SystemTime, UNIX_EPOCH};

use aws_sdk_dynamodb::types::AttributeValue;
use rand::Rng;

#[tokio::main]
async fn main() {
    let keys_table =
        std::env::var("KEYS_TABLE").unwrap_or_else(|_| "heartbeat-api-keys".to_string());

    let description = if let Some(desc) = parse_description() {
        desc
    } else {
        eprint!("--description is mandatory");
        std::process::exit(1);
    };

    // Generate a 32-byte random key and hex-encode it to 64 characters
    let random_bytes: [u8; 32] = rand::rng().random();
    let api_key: String = random_bytes.iter().map(|b| format!("{b:02x}")).collect();

    // Initialize AWS SDK
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    // Build put_item request
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();

    let request = client
        .put_item()
        .table_name(&keys_table)
        .item("api_key", AttributeValue::S(api_key.clone()))
        .item("created_at", AttributeValue::N(created_at.to_string()))
        .item("description", AttributeValue::S(description.clone()));

    if let Err(e) = request.send().await {
        eprintln!("Failed to store API key in DynamoDB: {e}");
        std::process::exit(1);
    }

    println!("New API key: {api_key} [{description}]]");
}

/// Parse `--description <value>` from CLI arguments.
fn parse_description() -> Option<String> {
    // Parse optional --description argument
    let args: Vec<String> = std::env::args().collect();

    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--description" {
            return iter.next().cloned();
        }
    }
    None
}
