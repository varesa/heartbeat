use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use heartbeat_core::{Monitor, MonitorStatus, Slug};

use crate::auth::{ApiError, ApiKey};
use crate::interval::{parse_interval, MAX_INTERVAL, MIN_INTERVAL};
use crate::state::AppState;

/// Default heartbeat interval: 5 minutes.
const DEFAULT_INTERVAL_SECS: u64 = 300;

/// TTL: 90 days in seconds.
const TTL_SECS: i64 = 90 * 24 * 60 * 60;

#[derive(Deserialize)]
pub struct HeartbeatQuery {
    pub interval: Option<String>,
}

#[derive(Serialize)]
pub struct HeartbeatResponse {
    pub ok: bool,
    pub next_due: String,
    pub status: MonitorStatus,
}

#[derive(Serialize)]
pub struct FailResponse {
    pub ok: bool,
    pub status: MonitorStatus,
}

#[derive(Serialize)]
pub struct MonitorListItem {
    pub slug: String,
    pub status: MonitorStatus,
    pub last_ping: i64,
    pub next_due: i64,
}

#[derive(Serialize)]
pub struct MonitorListResponse {
    pub monitors: Vec<MonitorListItem>,
}

/// GET /heartbeat/{slug}?interval=5m
///
/// Records a heartbeat ping. Creates the monitor on first ping.
/// If `?interval` is omitted and the monitor already exists, the existing
/// interval is preserved. If the monitor does not exist and no interval is
/// given, defaults to 5 minutes.
pub async fn heartbeat_handler(
    State(state): State<AppState>,
    _api_key: ApiKey,
    Path(slug_str): Path<String>,
    Query(query): Query<HeartbeatQuery>,
) -> Result<Json<HeartbeatResponse>, ApiError> {
    // Validate slug
    let slug = Slug::new(&slug_str).map_err(|e| ApiError::InvalidSlug(e.to_string()))?;

    // Determine interval
    let interval_secs = match &query.interval {
        Some(interval_str) => {
            let duration = parse_interval(interval_str).ok_or_else(|| {
                ApiError::InvalidInterval(format!("Cannot parse interval: {interval_str}"))
            })?;

            // Validate bounds
            if duration < MIN_INTERVAL {
                return Err(ApiError::InvalidInterval(format!(
                    "Interval too short: minimum is 30s, got {}s",
                    duration.as_secs()
                )));
            }
            if duration > MAX_INTERVAL {
                return Err(ApiError::InvalidInterval(format!(
                    "Interval too long: maximum is 365d, got {}s",
                    duration.as_secs()
                )));
            }

            duration.as_secs()
        }
        None => {
            // No interval specified: check if monitor already exists
            match state.monitors_store.get_monitor(&slug).await? {
                Some(existing) => existing.interval_secs,
                None => DEFAULT_INTERVAL_SECS,
            }
        }
    };

    let now = Utc::now().timestamp();
    let next_due = now + interval_secs as i64;

    let monitor = Monitor {
        slug: slug.to_string(),
        interval_secs,
        last_ping: now,
        next_due,
        check_partition: "CHECK".to_string(),
        last_alerted_at: None,
        alert_count: None,
        created_at: now,
        paused: None,
        expires_at: now + TTL_SECS,
    };

    state.monitors_store.upsert_monitor(&monitor).await?;

    let status = MonitorStatus::derive(&monitor, now);
    let next_due_str = chrono::DateTime::from_timestamp(next_due, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| next_due.to_string());

    Ok(Json(HeartbeatResponse {
        ok: true,
        next_due: next_due_str,
        status,
    }))
}

/// POST /heartbeat/{slug}/fail
///
/// Immediately marks a monitor as overdue by setting `next_due = 0`.
/// Creates the monitor in overdue state if it does not exist.
pub async fn fail_handler(
    State(state): State<AppState>,
    _api_key: ApiKey,
    Path(slug_str): Path<String>,
) -> Result<Json<FailResponse>, ApiError> {
    // Validate slug
    let slug = Slug::new(&slug_str).map_err(|e| ApiError::InvalidSlug(e.to_string()))?;

    let now = Utc::now().timestamp();

    // Determine interval: use existing if present, else default
    let interval_secs = match state.monitors_store.get_monitor(&slug).await? {
        Some(existing) => existing.interval_secs,
        None => DEFAULT_INTERVAL_SECS,
    };

    let monitor = Monitor {
        slug: slug.to_string(),
        interval_secs,
        last_ping: now,
        next_due: 0, // Immediately overdue
        check_partition: "CHECK".to_string(),
        last_alerted_at: None,
        alert_count: None,
        created_at: now,
        paused: None,
        expires_at: now + TTL_SECS,
    };

    state.monitors_store.upsert_monitor(&monitor).await?;

    let status = MonitorStatus::derive(&monitor, now);

    Ok(Json(FailResponse {
        ok: true,
        status,
    }))
}

/// GET /monitors
///
/// Returns all monitors sorted alphabetically by slug.
pub async fn list_monitors_handler(
    State(state): State<AppState>,
    _api_key: ApiKey,
) -> Result<Json<MonitorListResponse>, ApiError> {
    let monitors = state.monitors_store.list_monitors().await?;
    let now = Utc::now().timestamp();

    let mut items: Vec<MonitorListItem> = monitors
        .iter()
        .map(|m| MonitorListItem {
            slug: m.slug.clone(),
            status: MonitorStatus::derive(m, now),
            last_ping: m.last_ping,
            next_due: m.next_due,
        })
        .collect();

    items.sort_by(|a, b| a.slug.cmp(&b.slug));

    Ok(Json(MonitorListResponse { monitors: items }))
}

/// DELETE /monitors/{slug}
///
/// Removes a monitor from DynamoDB. Returns 204 on success, 404 if not found.
pub async fn delete_monitor_handler(
    State(state): State<AppState>,
    _api_key: ApiKey,
    Path(slug_str): Path<String>,
) -> Result<StatusCode, ApiError> {
    let slug = Slug::new(&slug_str).map_err(|e| ApiError::InvalidSlug(e.to_string()))?;
    state.monitors_store.delete_monitor(&slug).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /monitors/{slug}/pause
///
/// Pauses a monitor, clearing alert state. Returns 204 on success, 404 if not found.
pub async fn pause_handler(
    State(state): State<AppState>,
    _api_key: ApiKey,
    Path(slug_str): Path<String>,
) -> Result<StatusCode, ApiError> {
    let slug = Slug::new(&slug_str).map_err(|e| ApiError::InvalidSlug(e.to_string()))?;
    state.monitors_store.set_paused(&slug, true).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /monitors/{slug}/unpause
///
/// Unpauses a monitor. Returns 204 on success, 404 if not found.
pub async fn unpause_handler(
    State(state): State<AppState>,
    _api_key: ApiKey,
    Path(slug_str): Path<String>,
) -> Result<StatusCode, ApiError> {
    let slug = Slug::new(&slug_str).map_err(|e| ApiError::InvalidSlug(e.to_string()))?;
    state.monitors_store.set_paused(&slug, false).await?;
    Ok(StatusCode::NO_CONTENT)
}
