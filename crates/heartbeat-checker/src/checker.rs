use std::collections::HashSet;

use chrono::Utc;
use heartbeat_core::{DynamoStore, MonitorStatus};
use tracing::{info, warn};

use crate::alerts;
use crate::errors::CheckerError;
use crate::telegram::TelegramClient;

/// Fixed repeat alert interval: 1 hour in seconds.
const REPEAT_ALERT_INTERVAL_SECS: i64 = 3600;

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
