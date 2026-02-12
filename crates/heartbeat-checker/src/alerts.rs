use chrono::{DateTime, Utc};

/// Characters that must be escaped in MarkdownV2 (outside of code spans).
const MARKDOWN_V2_SPECIAL: &[char] = &[
    '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
];

/// Escape text for Telegram MarkdownV2 format.
///
/// Characters inside backtick code spans do NOT need escaping, but all other
/// text in the message does.
pub fn escape_markdown_v2(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        if MARKDOWN_V2_SPECIAL.contains(&ch) {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}

/// Format a human-readable duration from seconds using humantime.
fn format_duration(secs: u64) -> String {
    if secs == 0 {
        return "0s".to_string();
    }
    humantime::format_duration(std::time::Duration::from_secs(secs)).to_string()
}

/// Format a UTC timestamp from epoch seconds.
fn format_time(epoch: i64) -> String {
    DateTime::<Utc>::from_timestamp(epoch, 0)
        .map(|dt| dt.format("%H:%M UTC").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Format an overdue alert message (first alert).
///
/// Example: OVERDUE: `my-job` | interval: 5m | last: 12:03 UTC | 7m late
pub fn format_overdue(slug: &str, interval_secs: u64, last_ping: i64, now: i64) -> String {
    let interval = format_duration(interval_secs);
    let last = format_time(last_ping);
    let late_secs = (now - last_ping).saturating_sub(interval_secs as i64);
    let late = format_duration(late_secs.max(0) as u64);

    let raw = format!(
        "\u{26a0}\u{fe0f} OVERDUE: `{slug}` | interval: {interval} | last: {last} | {late} late"
    );

    escape_around_code_spans(&raw)
}

/// Format a repeat alert message (still overdue, sent every hour).
///
/// Example: STILL OVERDUE: `my-job` | down 23m
pub fn format_repeat(slug: &str, total_downtime_secs: u64) -> String {
    let downtime = format_duration(total_downtime_secs);

    let raw = format!("\u{26a0}\u{fe0f} STILL OVERDUE: `{slug}` | down {downtime}");

    escape_around_code_spans(&raw)
}

/// Format a recovery message.
///
/// Example: RECOVERED: `my-job` (was down 23m)
pub fn format_recovery(slug: &str, downtime_secs: u64) -> String {
    let downtime = format_duration(downtime_secs);

    let raw = format!("\u{2705} RECOVERED: `{slug}` \\(was down {downtime}\\)");

    escape_around_code_spans(&raw)
}

/// Escape MarkdownV2 special characters in text OUTSIDE of backtick code spans.
///
/// Text inside backticks (e.g., `slug-name`) does not need escaping in MarkdownV2.
fn escape_around_code_spans(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    let mut in_code = false;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '`' {
            in_code = !in_code;
            result.push(ch);
        } else if ch == '\\' && !in_code {
            // Already escaped -- pass through as-is
            result.push(ch);
            if i + 1 < chars.len() {
                i += 1;
                result.push(chars[i]);
            }
        } else if in_code {
            // Inside code span -- no escaping needed
            result.push(ch);
        } else {
            // Outside code span -- escape special characters
            if MARKDOWN_V2_SPECIAL.contains(&ch) {
                result.push('\\');
            }
            result.push(ch);
        }

        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_markdown_v2() {
        assert_eq!(escape_markdown_v2("hello"), "hello");
        assert_eq!(escape_markdown_v2("a.b"), "a\\.b");
        assert_eq!(escape_markdown_v2("a_b*c"), "a\\_b\\*c");
    }

    #[test]
    fn test_format_overdue_contains_slug() {
        let msg = format_overdue("my-job", 300, 1000, 1600);
        assert!(msg.contains("`my-job`"));
        assert!(msg.contains("OVERDUE"));
    }

    #[test]
    fn test_format_repeat_contains_slug() {
        let msg = format_repeat("my-job", 1380);
        assert!(msg.contains("`my-job`"));
        assert!(msg.contains("STILL OVERDUE"));
    }

    #[test]
    fn test_format_recovery_contains_slug() {
        let msg = format_recovery("my-job", 1380);
        assert!(msg.contains("`my-job`"));
        assert!(msg.contains("RECOVERED"));
    }

    #[test]
    fn test_escape_around_code_spans_preserves_backtick_content() {
        let text = "hello `my-slug` world.end";
        let escaped = escape_around_code_spans(text);
        // Inside backticks: my-slug should NOT be escaped
        assert!(escaped.contains("`my-slug`"));
        // Outside backticks: the dot should be escaped
        assert!(escaped.contains("world\\.end"));
    }
}
