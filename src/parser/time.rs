//! Time parsing for relative and absolute date/time values

use crate::predicate_error::PredicateParseError;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime};

/// Parse a duration unit (e.g., "days", "hours", "d", "h")
fn parse_duration(
    number: i64,
    unit: &str,
    original: &str,
) -> Result<Duration, PredicateParseError> {
    match unit {
        "seconds" | "second" | "secs" | "sec" | "s" => Ok(Duration::seconds(number)),
        "minutes" | "minute" | "mins" | "min" | "m" => Ok(Duration::minutes(number)),
        "hours" | "hour" | "hrs" | "hr" | "h" => Ok(Duration::hours(number)),
        "days" | "day" | "d" => Ok(Duration::days(number)),
        "weeks" | "week" | "w" => Ok(Duration::weeks(number)),
        _ => Err(PredicateParseError::Temporal(format!(
            "{original}: unknown unit: {unit}"
        ))),
    }
}

/// Try to parse input as a relative time (e.g., "7d", "3.hours")
/// Input should already have the leading '-' stripped if present
fn parse_relative_time(input: &str, original: &str) -> Result<Duration, PredicateParseError> {
    // Try period format first: "7.days"
    if let Some((num_str, unit)) = input.split_once('.') {
        let number = num_str
            .parse::<i64>()
            .map_err(|_| PredicateParseError::Temporal(format!("{original}: invalid number")))?;
        return parse_duration(number, unit, original);
    }

    // Try compact format: "7days" or "7d"
    let digit_end = input
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| PredicateParseError::Temporal(format!("{original}: missing time unit")))?;

    let num_str = &input[..digit_end];
    let unit = &input[digit_end..];

    // Check if unit starts with '-' to avoid parsing "2024-12-31" as relative time
    if unit.starts_with('-') {
        return Err(PredicateParseError::Temporal(format!(
            "{original}: not a relative time"
        )));
    }

    if unit.is_empty() {
        return Err(PredicateParseError::Temporal(format!(
            "{original}: missing time unit"
        )));
    }

    let number = num_str
        .parse::<i64>()
        .map_err(|_| PredicateParseError::Temporal(format!("{original}: invalid number")))?;

    parse_duration(number, unit, original)
}

/// Try to parse input as an absolute date/time
fn parse_absolute_time(s: &str) -> Result<DateTime<Local>, PredicateParseError> {
    // Try YYYY-MM-DD format
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if let Some(time) = date.and_hms_opt(0, 0, 0) {
            if let chrono::LocalResult::Single(local_time) = time.and_local_timezone(Local) {
                return Ok(local_time);
            }
        }
        return Err(PredicateParseError::Temporal(format!("{s}: invalid date")));
    }

    // Try RFC3339 with timezone: 2024-12-31T23:59:59Z
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Local));
    }

    // Try ISO8601 without timezone: 2024-12-31T23:59:59
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        if let chrono::LocalResult::Single(local_time) = naive_dt.and_local_timezone(Local) {
            return Ok(local_time);
        }
    }

    Err(PredicateParseError::Temporal(format!(
        "{s}: invalid date/time format. Supported formats:\n  \
         - Relative: -7d, 3hours, 2.weeks\n  \
         - Date: 2024-12-31\n  \
         - DateTime: 2024-12-31T23:59:59 or 2024-12-31T23:59:59Z"
    )))
}

/// Parse a time value from a string
///
/// Supports:
/// - Relative times: "-7d", "3hours", "-2.weeks"
/// - Absolute dates: "2024-12-31"
/// - ISO8601 with timezone: "2024-12-31T23:59:59Z"
/// - ISO8601 without timezone: "2024-12-31T23:59:59"
pub fn parse_time_value(s: &str) -> Result<DateTime<Local>, PredicateParseError> {
    // Trim whitespace for better UX
    let s = s.trim();

    // Handle negative prefix for relative times
    let (is_past, input) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (false, s)
    };

    // Try parsing as relative time first
    match parse_relative_time(input, s) {
        Ok(duration) => {
            return if is_past {
                Ok(Local::now() - duration)
            } else {
                Ok(Local::now() + duration)
            };
        }
        Err(e) => {
            // If it failed but looks like it was meant to be a relative time,
            // return the relative time error instead of trying absolute time.
            // Relative times have format: digits + unit (like "7d", "3hours")
            // or digits + "." + unit (like "7.days")
            // We check if there are digits followed by letters (not a dash for dates)
            if let Some(digit_end) = input.find(|c: char| !c.is_ascii_digit()) {
                // Only consider it a relative time attempt if there were actually digits
                if digit_end > 0 {
                    let after_digits = &input[digit_end..];
                    // If after digits we have letters (not '-' for dates) or '.' for period syntax
                    let looks_like_relative = after_digits.starts_with('.')
                        || matches!(after_digits.chars().next(), Some(c) if c.is_ascii_alphabetic());
                    if looks_like_relative {
                        return Err(e);
                    }
                }
            }
            // Otherwise fall through to absolute time parsing
        }
    }

    // Fall back to absolute time
    parse_absolute_time(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_relative_time_seconds() {
        let result = parse_time_value("-5s").unwrap();
        let expected = Local::now() - Duration::seconds(5);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-10seconds").unwrap();
        let expected = Local::now() - Duration::seconds(10);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-1sec").unwrap();
        let expected = Local::now() - Duration::seconds(1);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_relative_time_minutes() {
        let result = parse_time_value("-5m").unwrap();
        let expected = Local::now() - Duration::minutes(5);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-10minutes").unwrap();
        let expected = Local::now() - Duration::minutes(10);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-1min").unwrap();
        let expected = Local::now() - Duration::minutes(1);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_relative_time_hours() {
        let result = parse_time_value("-3h").unwrap();
        let expected = Local::now() - Duration::hours(3);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-2hours").unwrap();
        let expected = Local::now() - Duration::hours(2);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-1hr").unwrap();
        let expected = Local::now() - Duration::hours(1);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_relative_time_days() {
        let result = parse_time_value("-7d").unwrap();
        let expected = Local::now() - Duration::days(7);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-3days").unwrap();
        let expected = Local::now() - Duration::days(3);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-1day").unwrap();
        let expected = Local::now() - Duration::days(1);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_relative_time_weeks() {
        let result = parse_time_value("-2w").unwrap();
        let expected = Local::now() - Duration::weeks(2);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-1weeks").unwrap();
        let expected = Local::now() - Duration::weeks(1);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-1week").unwrap();
        let expected = Local::now() - Duration::weeks(1);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_relative_time_period_syntax() {
        let result = parse_time_value("-7.days").unwrap();
        let expected = Local::now() - Duration::days(7);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-3.hours").unwrap();
        let expected = Local::now() - Duration::hours(3);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-2.weeks").unwrap();
        let expected = Local::now() - Duration::weeks(2);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-5.minutes").unwrap();
        let expected = Local::now() - Duration::minutes(5);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_relative_time_positive_future() {
        let result = parse_time_value("7d").unwrap();
        let expected = Local::now() + Duration::days(7);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("3.hours").unwrap();
        let expected = Local::now() + Duration::hours(3);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_absolute_date_simple() {
        let result = parse_time_value("2024-12-31").unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.month(), 12);
        assert_eq!(result.day(), 31);
        assert_eq!(result.hour(), 0);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);
    }

    #[test]
    fn test_absolute_date_edge_cases() {
        // Leap year
        let result = parse_time_value("2024-02-29").unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.month(), 2);
        assert_eq!(result.day(), 29);

        // Month boundaries
        let result = parse_time_value("2024-01-31").unwrap();
        assert_eq!(result.day(), 31);

        let result = parse_time_value("2024-04-30").unwrap();
        assert_eq!(result.day(), 30);
    }

    #[test]
    fn test_iso8601_with_timezone() {
        let result = parse_time_value("2024-12-31T23:59:59Z").unwrap();
        assert_eq!(result.naive_utc().year(), 2024);
        assert_eq!(result.naive_utc().month(), 12);
        assert_eq!(result.naive_utc().day(), 31);
        assert_eq!(result.naive_utc().hour(), 23);
        assert_eq!(result.naive_utc().minute(), 59);
        assert_eq!(result.naive_utc().second(), 59);

        let result = parse_time_value("2024-12-31T23:59:59+00:00").unwrap();
        assert_eq!(result.naive_utc().year(), 2024);
    }

    #[test]
    fn test_iso8601_without_timezone() {
        // This is the bug fix - should now work
        let result = parse_time_value("2024-12-31T23:59:59").unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.month(), 12);
        assert_eq!(result.day(), 31);
        assert_eq!(result.hour(), 23);
        assert_eq!(result.minute(), 59);
        assert_eq!(result.second(), 59);

        let result = parse_time_value("2020-01-01T00:00:00").unwrap();
        assert_eq!(result.year(), 2020);
        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 1);
        assert_eq!(result.hour(), 0);
    }

    #[test]
    fn test_date_not_parsed_as_relative_time() {
        // The "-" in "2024-12-31" shouldn't be interpreted as relative time
        let result = parse_time_value("2024-12-31").unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.month(), 12);
        assert_eq!(result.day(), 31);
    }

    #[test]
    fn test_invalid_format_error_message() {
        let result = parse_time_value("not-a-date");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = format!("{err}");
        assert!(err_msg.contains("Supported formats"));
        assert!(err_msg.contains("Relative"));
        assert!(err_msg.contains("Date"));
        assert!(err_msg.contains("DateTime"));
    }

    #[test]
    fn test_invalid_relative_time() {
        // Unknown unit
        let result = parse_time_value("-5xyz");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("unknown unit"));

        // Missing unit
        let result = parse_time_value("-5");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_date() {
        // Invalid day
        let result = parse_time_value("2024-02-30");
        assert!(result.is_err());

        // Invalid month
        let result = parse_time_value("2024-13-01");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_string() {
        let result = parse_time_value("");
        assert!(result.is_err());
    }

    #[test]
    fn test_all_time_unit_abbreviations() {
        // Seconds
        assert!(parse_time_value("-1s").is_ok());
        assert!(parse_time_value("-1sec").is_ok());
        assert!(parse_time_value("-1secs").is_ok());
        assert!(parse_time_value("-1second").is_ok());
        assert!(parse_time_value("-1seconds").is_ok());

        // Minutes
        assert!(parse_time_value("-1m").is_ok());
        assert!(parse_time_value("-1min").is_ok());
        assert!(parse_time_value("-1mins").is_ok());
        assert!(parse_time_value("-1minute").is_ok());
        assert!(parse_time_value("-1minutes").is_ok());

        // Hours
        assert!(parse_time_value("-1h").is_ok());
        assert!(parse_time_value("-1hr").is_ok());
        assert!(parse_time_value("-1hrs").is_ok());
        assert!(parse_time_value("-1hour").is_ok());
        assert!(parse_time_value("-1hours").is_ok());

        // Days
        assert!(parse_time_value("-1d").is_ok());
        assert!(parse_time_value("-1day").is_ok());
        assert!(parse_time_value("-1days").is_ok());

        // Weeks
        assert!(parse_time_value("-1w").is_ok());
        assert!(parse_time_value("-1week").is_ok());
        assert!(parse_time_value("-1weeks").is_ok());
    }

    #[test]
    fn test_mixed_valid_and_invalid_inputs() {
        // Valid formats should parse
        assert!(parse_time_value("2024-01-01").is_ok());
        assert!(parse_time_value("2024-01-01T12:00:00").is_ok());
        assert!(parse_time_value("2024-01-01T12:00:00Z").is_ok());
        assert!(parse_time_value("-7d").is_ok());
        assert!(parse_time_value("3.hours").is_ok());

        // Invalid formats should fail
        assert!(parse_time_value("2024/01/01").is_err());
        assert!(parse_time_value("01-01-2024").is_err());
        assert!(parse_time_value("12:00:00").is_err());
        assert!(parse_time_value("7 days ago").is_err());
    }

    #[test]
    fn test_zero_duration() {
        // Zero duration should be valid
        let result = parse_time_value("-0d").unwrap();
        let expected = Local::now();
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("0d").unwrap();
        let expected = Local::now();
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("0.days").unwrap();
        let expected = Local::now();
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("-0hours").unwrap();
        let expected = Local::now();
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        let result = parse_time_value("0.seconds").unwrap();
        let expected = Local::now();
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_whitespace_handling() {
        // Leading whitespace
        let result = parse_time_value(" -7d").unwrap();
        let expected = Local::now() - Duration::days(7);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        // Trailing whitespace
        let result = parse_time_value("-7d ").unwrap();
        let expected = Local::now() - Duration::days(7);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        // Both
        let result = parse_time_value("  -7d  ").unwrap();
        let expected = Local::now() - Duration::days(7);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        // Absolute dates with whitespace
        let result = parse_time_value(" 2024-12-31 ").unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.month(), 12);
        assert_eq!(result.day(), 31);

        // ISO8601 with whitespace
        let result = parse_time_value("  2024-12-31T23:59:59  ").unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.hour(), 23);
    }

    #[test]
    fn test_empty_unit_fails() {
        // Period without unit should fail
        let result = parse_time_value("-7.");
        assert!(result.is_err());

        let result = parse_time_value("7.");
        assert!(result.is_err());

        // Just period
        let result = parse_time_value("-.");
        assert!(result.is_err());

        let result = parse_time_value(".");
        assert!(result.is_err());
    }

    #[test]
    fn test_timezone_offset_formats() {
        // RFC3339 with positive offset
        let result = parse_time_value("2024-12-31T23:59:59+05:30");
        assert!(result.is_ok());

        // RFC3339 with negative offset
        let result = parse_time_value("2024-12-31T23:59:59-08:00");
        assert!(result.is_ok());
    }

    #[test]
    fn test_fractional_seconds_unsupported() {
        // Fractional seconds not currently supported - should fail gracefully
        let result = parse_time_value("2024-12-31T23:59:59.999");
        assert!(result.is_err());

        let result = parse_time_value("2024-12-31T23:59:59.999Z");
        // This might work via RFC3339
        // Just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_edge_case_numbers() {
        // Very large numbers (but not overflow)
        let result = parse_time_value("-99999d");
        assert!(result.is_ok());

        // Single digit
        let result = parse_time_value("-1d").unwrap();
        let expected = Local::now() - Duration::days(1);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);

        // Multiple digits
        let result = parse_time_value("-365d").unwrap();
        let expected = Local::now() - Duration::days(365);
        assert!((result.timestamp() - expected.timestamp()).abs() <= 1);
    }
}
