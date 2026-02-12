# Containerfile -- heartbeat-api
#
# Multi-stage build using cargo-chef for dependency caching.
# Build:  podman build -t heartbeat-api -f Containerfile .
# Run:    podman run -p 3000:3000 --env-file .env heartbeat-api
#
# Environment variables:
#   MONITORS_TABLE           - DynamoDB monitors table (default: "heartbeat-monitors")
#   KEYS_TABLE               - DynamoDB API keys table (default: "heartbeat-api-keys")
#   BIND_ADDR                - Listen address (default: "0.0.0.0:3000")
#   AWS_ACCESS_KEY_ID        - AWS credentials
#   AWS_SECRET_ACCESS_KEY    - AWS credentials
#   AWS_REGION               - AWS region for DynamoDB
#   RUST_LOG                 - Tracing filter (default: "info")

# ---- Stage 1: planner ----
# Compute a dependency recipe so rebuilds only recompile when Cargo.toml changes.
FROM docker.io/rust:1-bookworm AS planner
RUN cargo install cargo-chef --locked
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---- Stage 2: builder ----
# Cook dependencies first (cached layer), then compile the binary.
FROM docker.io/rust:1-bookworm AS builder
RUN cargo install cargo-chef --locked
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin heartbeat-api

# ---- Stage 3: runtime ----
# Minimal Debian image with only the compiled binary.
FROM docker.io/debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1001 app && useradd -u 1001 -g app -m app

COPY --from=builder --chown=app:app /app/target/release/heartbeat-api /usr/local/bin/heartbeat-api

USER app
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/heartbeat-api"]
