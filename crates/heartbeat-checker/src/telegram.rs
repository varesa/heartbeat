use std::time::Duration;

use serde::Serialize;
use tracing::{info, warn};

use crate::errors::TelegramError;

/// Telegram Bot API client for sending alert messages.
#[derive(Clone)]
pub struct TelegramClient {
    http: reqwest::Client,
    bot_token: String,
    chat_id: String,
}

#[derive(Serialize)]
struct SendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
}

/// Telegram API response (partial).
#[derive(serde::Deserialize)]
struct TelegramResponse {
    ok: bool,
    description: Option<String>,
}

impl TelegramClient {
    /// Create a new Telegram client.
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            bot_token,
            chat_id,
        }
    }

    /// Send a message using MarkdownV2 parse mode.
    async fn send_message(&self, text: &str) -> Result<(), TelegramError> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let body = SendMessageRequest {
            chat_id: &self.chat_id,
            text,
            parse_mode: "MarkdownV2",
        };

        let resp = self.http.post(&url).json(&body).send().await?;

        let status = resp.status();
        let response: TelegramResponse = resp.json().await?;

        if !response.ok {
            return Err(TelegramError::ApiError(format!(
                "status={status}, description={}",
                response.description.unwrap_or_default()
            )));
        }

        Ok(())
    }

    /// Send a message with retry (3 attempts with exponential backoff).
    ///
    /// Delays: 500ms, 2s, 5s between retries.
    pub async fn send_with_retry(&self, text: &str) -> Result<(), TelegramError> {
        let delays = [
            Duration::from_millis(500),
            Duration::from_secs(2),
            Duration::from_secs(5),
        ];

        let mut last_err = None;

        for (attempt, delay) in std::iter::once(&Duration::ZERO)
            .chain(delays.iter())
            .enumerate()
        {
            if attempt > 0 {
                warn!(attempt, "Telegram send failed, retrying after {delay:?}");
                tokio::time::sleep(*delay).await;
            }

            match self.send_message(text).await {
                Ok(()) => {
                    if attempt > 0 {
                        info!(attempt, "Telegram send succeeded after retry");
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.expect("at least one attempt was made"))
    }
}
