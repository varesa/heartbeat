use std::collections::HashSet;
use std::fmt;

use chrono::Utc;
use heartbeat_core::{CoreError, DynamoStore, MonitorStatus};
use tracing::{info, warn};

use crate::alerts;
use crate::telegram::{TelegramClient, TelegramError};

/// Fixed repeat alert interval: 1 hour in seconds.
const REPEAT_ALERT_INTERVAL_SECS: i64 = 3600;

/// Errors from the checker.
#[derive(Debug)]
pub enum CheckerError {
    /// Error from DynamoDB operations.
    Core(CoreError),
    /// Error from Telegram API.
    Telegram(TelegramError),
}

impl fmt::Display for CheckerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(e) => write!(f, "checker core error: {e}"),
            Self::Telegram(e) => write!(f, "checker telegram error: {e}"),
        }
    }
}

impl std::error::Error for CheckerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(e) => Some(e),
            Self::Telegram(e) => Some(e),
        }
    }
}

impl From<CoreError> for CheckerError {
    fn from(e: CoreError) -> Self {
        Self::Core(e)
    }
}

impl From<TelegramError> for CheckerError {
    fn from(e: TelegramError) -> Self {
        Self::Telegram(e)
    }
}

/// Run the heartbeat check cycle.
///
/// 1. Query overdue monitors via GSI
/// 2. Query monitors with active alerts (for recovery detection)
/// 3. For overdue monitors: send first alert or repeat (if 1h+ since last)
/// 4. For recovered monitors: send recovery notification and clear alert state
pub async fn check_monitors(
    store: &DynamoStore,
    telegram: &TelegramClient,
) -> Result<(), CheckerError> {
    let now = Utc::now().timestamp();

    // Query overdue monitors and monitors with active alerts in parallel
    let overdue = store.query_overdue(now).await?;
    let alerted = store.query_alerted().await?;

    info!(
        overdue_count = overdue.len(),
        alerted_count = alerted.len(),
        "check cycle start"
    );

    // Track which slugs are currently overdue for recovery detection
    let mut overdue_slugs: HashSet<String> = HashSet::new();

    // Process overdue monitors
    for monitor in &overdue {
        let status = MonitorStatus::derive(monitor, now);

        // Skip paused monitors (GSI may return them since it doesn't filter on paused)
        if status == MonitorStatus::Paused {
            info!(slug = %monitor.slug, "skipping paused monitor");
            continue;
        }

        overdue_slugs.insert(monitor.slug.clone());

        let alert_count = monitor.alert_count.unwrap_or(0);

        match monitor.last_alerted_at {
            None => {
                // First alert
                let msg = alerts::format_overdue(
                    &monitor.slug,
                    monitor.interval_secs,
                    monitor.last_ping,
                    now,
                );
                match telegram.send_with_retry(&msg).await {
                    Ok(()) => {
                        store
                            .update_alert_state(&monitor.slug, now, alert_count + 1)
                            .await?;
                        info!(slug = %monitor.slug, "sent first overdue alert");
                    }
                    Err(e) => {
                        // Don't update last_alert_at if Telegram is unreachable
                        warn!(
                            slug = %monitor.slug,
                            error = %e,
                            "failed to send first alert, will retry next cycle"
                        );
                    }
                }
            }
            Some(last_alert) => {
                // Check if enough time has passed for a repeat alert (1 hour)
                if now - last_alert >= REPEAT_ALERT_INTERVAL_SECS {
                    let total_downtime = (now - monitor.next_due).max(0) as u64;
                    let msg = alerts::format_repeat(&monitor.slug, total_downtime);
                    match telegram.send_with_retry(&msg).await {
                        Ok(()) => {
                            store
                                .update_alert_state(&monitor.slug, now, alert_count + 1)
                                .await?;
                            info!(
                                slug = %monitor.slug,
                                alert_count = alert_count + 1,
                                "sent repeat overdue alert"
                            );
                        }
                        Err(e) => {
                            warn!(
                                slug = %monitor.slug,
                                error = %e,
                                "failed to send repeat alert, will retry next cycle"
                            );
                        }
                    }
                }
            }
        }
    }

    // Process recoveries: monitors that had alerts but are no longer overdue
    for monitor in &alerted {
        if overdue_slugs.contains(&monitor.slug) {
            // Still overdue -- already handled above
            continue;
        }

        let status = MonitorStatus::derive(monitor, now);

        // If it's paused, don't send recovery (the operator paused it)
        if status == MonitorStatus::Paused {
            continue;
        }

        // Monitor recovered (was alerted, now OK)
        if let Some(last_alert) = monitor.last_alerted_at {
            let downtime = (now - last_alert).max(0) as u64;
            let msg = alerts::format_recovery(&monitor.slug, downtime);
            match telegram.send_with_retry(&msg).await {
                Ok(()) => {
                    store.clear_alert_state(&monitor.slug).await?;
                    info!(slug = %monitor.slug, "sent recovery notification");
                }
                Err(e) => {
                    warn!(
                        slug = %monitor.slug,
                        error = %e,
                        "failed to send recovery alert, will retry next cycle"
                    );
                }
            }
        }
    }

    info!("check cycle complete");
    Ok(())
}
