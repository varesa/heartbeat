# Heartbeat

A dead man's switch monitoring service. Services, cron jobs, and background processes prove they are alive by sending periodic HTTP pings. If check-ins are missed, alerts are sent to Telegram.

## Architecture

Heartbeat is split into two runtime components backed by a shared core library:

```
On-Prem             AWS
┌──────────────┐                        ┌──────────────┐
│  heartbeat-  │                        │  heartbeat-  │
│  api         │                        │  checker     │
│              │                        │              │
│  Axum server │    ┌──────────────┐    │  Lambda      │
│  in k8s      │───►│  DynamoDB    │◄───│  (every 2m)  │
│              │    │  - monitors  │    │              │
│              │    │  - api-keys  │    │       alerts │──► Telegram
└──────────────┘    └──────────────┘    └──────────────┘
                                               ▲
                                               │ SSM Parameter Store
                                               │ (bot token, chat id)
```

**heartbeat-api** runs on-premises in a container. It receives heartbeat pings over HTTP, validates API keys, and writes monitor state to DynamoDB.

**heartbeat-checker** runs as an AWS Lambda on a schedule. It queries a DynamoDB table for overdue monitors, sends Telegram alerts for overdue monitors (repeating hourly) and recovery notifications when monitors come back.

**heartbeat-core** is the shared library containing the data models and DynamoDB access code

### Monitor lifecycle

```
         heartbeat received
                │
                ▼
               ok ◄──────── recovery (alerts cleared)
              ╱               ▲
     next_due passes          │ heartbeat received
              ╲               │
               ▼              │
            overdue ──► repeat alerts every 1h
```

Monitors are created automatically on first ping. They expire via DynamoDB TTL 90 days after the last ping.

### Crate structure

```
crates/
├── heartbeat-core/       Shared: Monitor model, DynamoStore, etc.
├── heartbeat-api/        HTTP API server
└── heartbeat-checker/    Lambda checker + Telegram alerter
```

## API

All endpoints require `Authorization: Bearer <api_key>`.

| Method   | Path                          | Description                         |
|----------|-------------------------------|-------------------------------------|
| `GET`    | `/heartbeat/{slug}?interval=` | Record a ping (creates on first use)|
| `POST`   | `/heartbeat/{slug}/fail`      | Immediately mark as overdue         |
| `GET`    | `/monitors`                   | List all monitors with status       |
| `DELETE` | `/monitors/{slug}`            | Remove a monitor                    |
| `POST`   | `/monitors/{slug}/pause`      | Pause alerting                      |
| `POST`   | `/monitors/{slug}/unpause`    | Resume alerting                     |

**Slug rules:** 1-64 chars, lowercase alphanumeric and hyphens, no leading/trailing hyphens.

**Interval format:** Human-readable durations (`5m`, `1h`, `2h30m`) or raw seconds. Range: 30s to 365d. Defaults to 5 minutes if omitted on first ping.

### Example usage

```bash
# Send a heartbeat every 5 minutes
curl -H "Authorization: Bearer $API_KEY" \
  "https://heartbeat.example.com/heartbeat/nightly-backup?interval=5m"

# Signal explicit failure
curl -X POST -H "Authorization: Bearer $API_KEY" \
  "https://heartbeat.example.com/heartbeat/nightly-backup/fail"

# List monitors
curl -H "Authorization: Bearer $API_KEY" \
  "https://heartbeat.example.com/monitors"
```

## Prerequisites

- Rust toolchain (2024 edition)
- [cargo-lambda](https://www.cargo-lambda.info/) for building the Lambda
- Zig for cross-compiling arm64 lambda binaries
- Terraform for infrastructure
- A Telegram bot token and chat ID for alerts

## Building

### API (container)

```bash
podman build -t heartbeat-api -f Containerfile .
```

### Lambda (zip)

```bash
./scripts/build-lambda.sh
# Output: target/lambda/heartbeat-checker/bootstrap.zip
```

### Generate an API key

```bash
cargo run --bin add-api-key -- --description "my service"
```

This creates a random key and stores it in the `heartbeat-api-keys` DynamoDB table.

## Deploying

### 1. Provision infrastructure

```bash
cd terraform

terraform init

terraform apply \
  -var="telegram_bot_token=YOUR_BOT_TOKEN" \
  -var="telegram_chat_id=YOUR_CHAT_ID" \
  -var="alert_email=you@example.com"
```

This creates:
- DynamoDB tables (`heartbeat-monitors` with overdue-check GSI, `heartbeat-api-keys`)
- Lambda function with EventBridge 2-minute schedule
- SSM parameters for Telegram secrets
- IAM roles and policies
- CloudWatch log group (14-day retention)

Terraform state is stored in S3 (configured in `terraform/main.tf`).

### 2. Deploy the Lambda

The Lambda zip is uploaded by Terraform from `target/lambda/heartbeat-checker/bootstrap.zip`. Build it before running `terraform apply`:

```bash
./scripts/build-lambda.sh
terraform -chdir=terraform apply
```

### 3. Run the API

```bash
podman run -d --name heartbeat-api \
  -p 3000:3000 \
  -e AWS_ACCESS_KEY_ID=... \
  -e AWS_SECRET_ACCESS_KEY=... \
  -e AWS_REGION=eu-north-1 \
  -e MONITORS_TABLE=heartbeat-monitors \
  -e KEYS_TABLE=heartbeat-api-keys \
  heartbeat-api
```

### Environment variables

**API:**

| Variable                | Default                | Description                    |
|-------------------------|------------------------|--------------------------------|
| `MONITORS_TABLE`        | `heartbeat-monitors`   | DynamoDB monitors table        |
| `KEYS_TABLE`            | `heartbeat-api-keys`   | DynamoDB API keys table        |
| `BIND_ADDR`             | `0.0.0.0:3000`         | Listen address                 |
| `AWS_ACCESS_KEY_ID`     | --                     | AWS credentials                |
| `AWS_SECRET_ACCESS_KEY` | --                     | AWS credentials                |
| `AWS_REGION`            | --                     | AWS region                     |
| `RUST_LOG`              | `info`                 | Log level filter               |

**Lambda** (set by Terraform):

| Variable                         | Description                     |
|----------------------------------|---------------------------------|
| `HEARTBEAT_TABLE_NAME`           | DynamoDB monitors table         |
| `HEARTBEAT_API_KEYS_TABLE_NAME`  | DynamoDB API keys table         |
| `TELEGRAM_BOT_TOKEN_PARAM`       | SSM parameter path for bot token|
| `TELEGRAM_CHAT_ID_PARAM`         | SSM parameter path for chat ID  |

## AWS resources

| Service              | Purpose                              | Config                      |
|----------------------|--------------------------------------|-----------------------------|
| DynamoDB             | Monitor and API key storage          | On-demand capacity          |
| Lambda               | Periodic overdue checker + alerter   | arm64, 128 MB, concurrency 1|
| EventBridge          | Lambda trigger                       | Every 2 minutes             |
| SSM Parameter Store  | Telegram secrets                     | SecureString                |
| CloudWatch Logs      | Lambda logs                          | 14-day retention            |
| S3                   | Terraform state                      |                             |
