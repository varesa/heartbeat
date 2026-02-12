use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Slug
// ---------------------------------------------------------------------------

const MAX_SLUG_LENGTH: usize = 64;

/// A validated monitor slug: lowercase alphanumeric characters and hyphens,
/// 1-64 characters, no leading or trailing hyphen.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Slug(String);

impl Slug {
    /// Create a new `Slug` after validation.
    pub fn new(s: impl Into<String>) -> Result<Self, SlugError> {
        let s = s.into();

        if s.is_empty() {
            return Err(SlugError::Empty);
        }

        if s.len() > MAX_SLUG_LENGTH {
            return Err(SlugError::TooLong(s.len()));
        }

        if !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(SlugError::InvalidCharacters);
        }

        if s.starts_with('-') || s.ends_with('-') {
            return Err(SlugError::InvalidHyphenPosition);
        }

        Ok(Self(s))
    }
}

impl TryFrom<String> for Slug {
    type Error = SlugError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Slug> for String {
    fn from(slug: Slug) -> Self {
        slug.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Slug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Errors that can occur when validating a slug.
#[derive(Debug, Clone, Error)]
pub enum SlugError {
    #[error("slug must not be empty")]
    Empty,

    #[error("slug length {0} exceeds maximum of {MAX_SLUG_LENGTH}")]
    TooLong(usize),

    #[error("slug must contain only lowercase letters, digits, and hyphens")]
    InvalidCharacters,

    #[error("slug must not start or end with a hyphen")]
    InvalidHyphenPosition,
}

// ---------------------------------------------------------------------------
// Monitor
// ---------------------------------------------------------------------------

/// A heartbeat monitor stored in DynamoDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    /// Unique identifier (validated as a Slug at API boundaries).
    pub slug: String,

    /// Expected check-in interval in seconds.
    pub interval_secs: u64,

    /// Unix epoch seconds of the last received ping.
    pub last_ping: i64,

    /// Unix epoch seconds when the monitor becomes overdue.
    pub next_due: i64,

    /// Fixed partition key for the GSI (always "CHECK").
    pub check_partition: String,

    /// Unix epoch seconds of the last alert sent (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_alerted_at: Option<i64>,

    /// Number of consecutive alerts sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_count: Option<u32>,

    /// Unix epoch seconds when this monitor was created.
    pub created_at: i64,

    /// Whether this monitor is paused (explicit state).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,

    /// TTL: last_ping + 90 days (in seconds). DynamoDB auto-deletes after this.
    pub expires_at: i64,
}

// ---------------------------------------------------------------------------
// MonitorStatus (derived, not stored)
// ---------------------------------------------------------------------------

/// Derived monitor status -- not stored in DynamoDB.
///
/// - `Paused`: `monitor.paused == Some(true)`
/// - `Overdue`: `monitor.next_due < now`
/// - `Ok`: otherwise
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MonitorStatus {
    Ok,
    Overdue,
    Paused,
}

impl MonitorStatus {
    /// Derive the status of a monitor at a given point in time.
    pub fn derive(monitor: &Monitor, now_epoch: i64) -> Self {
        if monitor.paused == Some(true) {
            Self::Paused
        } else if monitor.next_due < now_epoch {
            Self::Overdue
        } else {
            Self::Ok
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Slug tests --

    #[test]
    fn valid_slugs() {
        assert!(Slug::new("my-service").is_ok());
        assert!(Slug::new("a").is_ok());
        assert!(Slug::new("abc123").is_ok());
        assert!(Slug::new("nightly-backup-2").is_ok());
        assert!(Slug::new("a".repeat(64)).is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(Slug::new(""), Err(SlugError::Empty)));
    }

    #[test]
    fn rejects_too_long() {
        let long = "a".repeat(65);
        assert!(matches!(Slug::new(long), Err(SlugError::TooLong(65))));
    }

    #[test]
    fn rejects_uppercase() {
        assert!(matches!(Slug::new("Hello"), Err(SlugError::InvalidCharacters)));
    }

    #[test]
    fn rejects_spaces() {
        assert!(matches!(Slug::new("my service"), Err(SlugError::InvalidCharacters)));
    }

    #[test]
    fn rejects_leading_hyphen() {
        assert!(matches!(Slug::new("-service"), Err(SlugError::InvalidHyphenPosition)));
    }

    #[test]
    fn rejects_trailing_hyphen() {
        assert!(matches!(Slug::new("service-"), Err(SlugError::InvalidHyphenPosition)));
    }

    #[test]
    fn display_and_as_ref() {
        let slug = Slug::new("test-slug").unwrap();
        assert_eq!(slug.to_string(), "test-slug");
        assert_eq!(slug.as_ref(), "test-slug");
    }

    #[test]
    fn roundtrip_string_conversion() {
        let slug = Slug::new("my-slug").unwrap();
        let s: String = slug.clone().into();
        let back: Slug = s.try_into().unwrap();
        assert_eq!(slug, back);
    }

    // -- MonitorStatus tests --

    fn make_monitor(next_due: i64, paused: Option<bool>) -> Monitor {
        Monitor {
            slug: "test".into(),
            interval_secs: 300,
            last_ping: 1000,
            next_due,
            check_partition: "CHECK".into(),
            last_alerted_at: None,
            alert_count: None,
            created_at: 1000,
            paused,
            expires_at: 1000 + 90 * 86400,
        }
    }

    #[test]
    fn status_ok() {
        let m = make_monitor(2000, None);
        assert_eq!(MonitorStatus::derive(&m, 1500), MonitorStatus::Ok);
    }

    #[test]
    fn status_overdue() {
        let m = make_monitor(1000, None);
        assert_eq!(MonitorStatus::derive(&m, 1500), MonitorStatus::Overdue);
    }

    #[test]
    fn status_paused() {
        let m = make_monitor(500, Some(true));
        // Paused takes precedence over overdue
        assert_eq!(MonitorStatus::derive(&m, 1500), MonitorStatus::Paused);
    }

    #[test]
    fn status_paused_false_is_ok() {
        let m = make_monitor(2000, Some(false));
        assert_eq!(MonitorStatus::derive(&m, 1500), MonitorStatus::Ok);
    }
}
