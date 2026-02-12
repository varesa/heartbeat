use std::time::Duration;

/// Parse an interval string into a [`Duration`].
///
/// Accepts either:
/// - Human-readable shorthand via `humantime` (e.g. "5m", "1h", "30s", "2h30m")
/// - Raw seconds as a plain integer (e.g. "300")
pub fn parse_interval(s: &str) -> Option<Duration> {
    // Try humantime first (handles "5m", "1h30m", "30s", etc.)
    if let Ok(d) = humantime::parse_duration(s) {
        return Some(d);
    }

    // Fallback: try parsing as raw seconds
    if let Ok(secs) = s.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }

    None
}

/// Minimum allowed interval: 30 seconds.
pub const MIN_INTERVAL: Duration = Duration::from_secs(30);

/// Maximum allowed interval: 365 days.
pub const MAX_INTERVAL: Duration = Duration::from_secs(365 * 24 * 60 * 60);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_humantime_minutes() {
        assert_eq!(parse_interval("5m"), Some(Duration::from_secs(300)));
    }

    #[test]
    fn parse_humantime_hours() {
        assert_eq!(parse_interval("1h"), Some(Duration::from_secs(3600)));
    }

    #[test]
    fn parse_humantime_seconds() {
        assert_eq!(parse_interval("30s"), Some(Duration::from_secs(30)));
    }

    #[test]
    fn parse_humantime_compound() {
        assert_eq!(
            parse_interval("1h30m"),
            Some(Duration::from_secs(3600 + 1800))
        );
    }

    #[test]
    fn parse_raw_seconds() {
        assert_eq!(parse_interval("300"), Some(Duration::from_secs(300)));
    }

    #[test]
    fn parse_invalid() {
        assert_eq!(parse_interval("foo"), None);
        assert_eq!(parse_interval(""), None);
        assert_eq!(parse_interval("-5"), None);
    }
}
