mod auth;
mod interval;
mod routes;
mod state;

use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use heartbeat_core::DynamoStore;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Configuration from environment
    let monitors_table =
        std::env::var("MONITORS_TABLE").unwrap_or_else(|_| "heartbeat-monitors".to_string());
    let keys_table =
        std::env::var("KEYS_TABLE").unwrap_or_else(|_| "heartbeat-api-keys".to_string());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    tracing::info!(monitors_table, keys_table, bind_addr, "Starting heartbeat-api");

    // Initialize DynamoDB store
    let monitors_store = DynamoStore::new(&monitors_table).await;

    // Share the underlying DynamoDB client for key lookups
    let dynamo_client = monitors_store.client().clone();

    let state = AppState {
        monitors_store,
        dynamo_client,
        keys_table,
    };

    // Build router
    let app = Router::new()
        .route(
            "/heartbeat/{slug}",
            axum::routing::get(routes::heartbeat_handler),
        )
        .route(
            "/heartbeat/{slug}/fail",
            axum::routing::post(routes::fail_handler),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Bind and serve
    let addr: SocketAddr = bind_addr.parse().expect("Invalid BIND_ADDR");
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    tracing::info!(%addr, "Listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

/// Wait for SIGTERM or SIGINT for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Ctrl+C received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
