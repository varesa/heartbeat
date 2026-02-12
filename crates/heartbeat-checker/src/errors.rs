use heartbeat_core::CoreError;

/// Errors from the Telegram API.
#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    /// HTTP transport error.
    #[error("Telegram HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// Telegram API returned a non-ok response.
    #[error("Telegram API error: {0}")]
    ApiError(String),
}

/// Errors from the checker.
#[derive(Debug, thiserror::Error)]
pub enum CheckerError {
    /// Error from DynamoDB operations.
    #[error("checker core error: {0}")]
    Core(#[from] CoreError),
    /// Error from Telegram API.
    #[error("checker telegram error: {0}")]
    Telegram(#[from] TelegramError),
}
