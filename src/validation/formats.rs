use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;

use crate::schemas::StringFormat;

/// Validates a string value against a `StringFormat`.
///
/// Returns `None` if valid, or `Some(error_message)` if invalid.
/// Unknown formats always pass (annotation-only).
pub fn validate_format(format: &StringFormat, value: &str) -> Option<String> {
    let valid = match format {
        StringFormat::DateTime => is_valid_date_time(value),
        StringFormat::Date => is_valid_date(value),
        StringFormat::Time => is_valid_time(value),
        StringFormat::Duration => is_valid_duration(value),
        StringFormat::Email => is_valid_email(value),
        StringFormat::IdnEmail => is_valid_email(value),
        StringFormat::Hostname => is_valid_hostname(value),
        StringFormat::IdnHostname => true,
        StringFormat::Ipv4 => is_valid_ipv4(value),
        StringFormat::Ipv6 => is_valid_ipv6(value),
        StringFormat::Uri => is_valid_uri(value),
        StringFormat::UriReference => is_valid_uri_reference(value),
        StringFormat::Iri => is_valid_uri(value),
        StringFormat::IriReference => is_valid_uri_reference(value),
        StringFormat::Uuid => is_valid_uuid(value),
        StringFormat::UriTemplate => is_valid_uri_template(value),
        StringFormat::JsonPointer => is_valid_json_pointer(value),
        StringFormat::RelativeJsonPointer => is_valid_relative_json_pointer(value),
        StringFormat::Regex => is_valid_regex(value),
        StringFormat::Unknown(_) => true,
    };

    if valid {
        None
    } else {
        Some(format!("String \"{value}\" is not a valid \"{format}\"",))
    }
}

// --- Date/Time (RFC 3339) ---

static DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$").expect("DATE_RE"));

static TIME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{2}):(\d{2}):(\d{2})(\.\d+)?(Z|[+-]\d{2}:\d{2})$").expect("TIME_RE")
});

static DATETIME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[Tt](\d{2}):(\d{2}):(\d{2})(\.\d+)?(Z|[+-]\d{2}:\d{2})$")
        .expect("DATETIME_RE")
});

fn is_valid_date(value: &str) -> bool {
    let Some(caps) = DATE_RE.captures(value) else {
        return false;
    };
    let year: u32 = caps[1].parse().unwrap_or(0);
    let month: u32 = caps[2].parse().unwrap_or(0);
    let day: u32 = caps[3].parse().unwrap_or(0);
    is_valid_calendar_date(year, month, day)
}

fn is_valid_time(value: &str) -> bool {
    let Some(caps) = TIME_RE.captures(value) else {
        return false;
    };
    let hour: u32 = caps[1].parse().unwrap_or(99);
    let minute: u32 = caps[2].parse().unwrap_or(99);
    let second: u32 = caps[3].parse().unwrap_or(99);
    hour <= 23 && minute <= 59 && second <= 60
}

fn is_valid_date_time(value: &str) -> bool {
    let Some(caps) = DATETIME_RE.captures(value) else {
        return false;
    };
    let year: u32 = caps[1].parse().unwrap_or(0);
    let month: u32 = caps[2].parse().unwrap_or(0);
    let day: u32 = caps[3].parse().unwrap_or(0);
    let hour: u32 = caps[4].parse().unwrap_or(99);
    let minute: u32 = caps[5].parse().unwrap_or(99);
    let second: u32 = caps[6].parse().unwrap_or(99);
    is_valid_calendar_date(year, month, day) && hour <= 23 && minute <= 59 && second <= 60
}

fn is_valid_calendar_date(year: u32, month: u32, day: u32) -> bool {
    if !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    day <= max_day
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

// --- Duration (ISO 8601) ---

static DURATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^P(\d+Y)?(\d+M)?(\d+W)?(\d+D)?(T(\d+H)?(\d+M)?(\d+(\.\d+)?S)?)?$")
        .expect("DURATION_RE")
});

fn is_valid_duration(value: &str) -> bool {
    if !DURATION_RE.is_match(value) {
        return false;
    }
    // "P" alone is not valid; must have at least one component
    if value == "P" || value == "PT" {
        return false;
    }
    true
}

// --- Email (simplified RFC 5321) ---

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").expect("EMAIL_RE"));

fn is_valid_email(value: &str) -> bool {
    EMAIL_RE.is_match(value)
}

// --- Hostname (RFC 1123) ---

fn is_valid_hostname(value: &str) -> bool {
    if value.is_empty() || value.len() > 253 {
        return false;
    }
    let value = value.strip_suffix('.').unwrap_or(value);
    for label in value.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
    }
    true
}

// --- IPv4/IPv6 ---

fn is_valid_ipv4(value: &str) -> bool {
    Ipv4Addr::from_str(value).is_ok()
}

fn is_valid_ipv6(value: &str) -> bool {
    Ipv6Addr::from_str(value).is_ok()
}

// --- URI / URI-reference ---

fn is_valid_uri(value: &str) -> bool {
    url::Url::parse(value).is_ok()
}

fn is_valid_uri_reference(value: &str) -> bool {
    if is_valid_uri(value) {
        return true;
    }
    // A relative reference: must not contain spaces, and either starts with
    // a path segment, query, or fragment.
    !value.contains(' ') && !value.is_empty()
}

// --- UUID (RFC 4122) ---

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
        .expect("UUID_RE")
});

fn is_valid_uuid(value: &str) -> bool {
    UUID_RE.is_match(value)
}

// --- URI Template (RFC 6570 basic check) ---

fn is_valid_uri_template(value: &str) -> bool {
    let mut depth = 0i32;
    for ch in value.chars() {
        match ch {
            '{' => {
                depth += 1;
                if depth > 1 {
                    return false;
                }
            }
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

// --- JSON Pointer (RFC 6901) ---

fn is_valid_json_pointer(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    if !value.starts_with('/') {
        return false;
    }
    // Validate escape sequences: ~ must be followed by 0 or 1
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '~' {
            match chars.peek() {
                Some('0') | Some('1') => {
                    chars.next();
                }
                _ => return false,
            }
        }
    }
    true
}

// --- Relative JSON Pointer ---

fn is_valid_relative_json_pointer(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    // Must start with a non-negative integer
    let rest = value.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.len() == value.len() {
        return false;
    }
    // Leading zeros are not allowed (except "0" itself)
    let int_part = &value[..value.len() - rest.len()];
    if int_part.len() > 1 && int_part.starts_with('0') {
        return false;
    }
    // After the integer: either '#' or a valid JSON pointer, or nothing
    if rest.is_empty() || rest == "#" {
        return true;
    }
    is_valid_json_pointer(rest)
}

// --- Regex (ECMA 262) ---

fn is_valid_regex(value: &str) -> bool {
    regex::Regex::new(value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- date ---

    #[test]
    fn test_valid_dates() {
        assert!(is_valid_date("2024-01-15"));
        assert!(is_valid_date("2000-02-29"));
        assert!(is_valid_date("1970-01-01"));
    }

    #[test]
    fn test_invalid_dates() {
        assert!(!is_valid_date("not-a-date"));
        assert!(!is_valid_date("2024-13-01"));
        assert!(!is_valid_date("2024-02-30"));
        assert!(!is_valid_date("2023-02-29"));
        assert!(!is_valid_date("2024-00-01"));
        assert!(!is_valid_date("2024-01-00"));
    }

    // --- time ---

    #[test]
    fn test_valid_times() {
        assert!(is_valid_time("12:00:00Z"));
        assert!(is_valid_time("23:59:59Z"));
        assert!(is_valid_time("00:00:00+00:00"));
        assert!(is_valid_time("12:30:00.123Z"));
        assert!(is_valid_time("23:59:60Z")); // leap second
    }

    #[test]
    fn test_invalid_times() {
        assert!(!is_valid_time("25:00:00Z"));
        assert!(!is_valid_time("12:60:00Z"));
        assert!(!is_valid_time("12:00:61Z"));
        assert!(!is_valid_time("not-a-time"));
        assert!(!is_valid_time("12:00:00")); // missing timezone
    }

    // --- date-time ---

    #[test]
    fn test_valid_datetimes() {
        assert!(is_valid_date_time("2024-01-15T12:00:00Z"));
        assert!(is_valid_date_time("2024-01-15t12:00:00Z"));
        assert!(is_valid_date_time("2024-01-15T12:00:00+05:30"));
        assert!(is_valid_date_time("2024-01-15T12:00:00.123Z"));
    }

    #[test]
    fn test_invalid_datetimes() {
        assert!(!is_valid_date_time("2024-01-15"));
        assert!(!is_valid_date_time("not-a-datetime"));
        assert!(!is_valid_date_time("2024-13-01T12:00:00Z"));
    }

    // --- duration ---

    #[test]
    fn test_valid_durations() {
        assert!(is_valid_duration("P1Y"));
        assert!(is_valid_duration("P1Y2M3D"));
        assert!(is_valid_duration("PT1H"));
        assert!(is_valid_duration("PT1H30M"));
        assert!(is_valid_duration("P1Y2M3DT4H5M6S"));
        assert!(is_valid_duration("P1W"));
        assert!(is_valid_duration("PT0.5S"));
    }

    #[test]
    fn test_invalid_durations() {
        assert!(!is_valid_duration("P"));
        assert!(!is_valid_duration("PT"));
        assert!(!is_valid_duration("not-a-duration"));
        assert!(!is_valid_duration("1Y"));
    }

    // --- email ---

    #[test]
    fn test_valid_emails() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("user+tag@sub.example.com"));
    }

    #[test]
    fn test_invalid_emails() {
        assert!(!is_valid_email("not-an-email"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user @example.com"));
    }

    // --- hostname ---

    #[test]
    fn test_valid_hostnames() {
        assert!(is_valid_hostname("example.com"));
        assert!(is_valid_hostname("sub.example.com"));
        assert!(is_valid_hostname("localhost"));
        assert!(is_valid_hostname("a"));
    }

    #[test]
    fn test_invalid_hostnames() {
        assert!(!is_valid_hostname("-invalid.com"));
        assert!(!is_valid_hostname("invalid-.com"));
        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("exam ple.com"));
    }

    // --- ipv4 ---

    #[test]
    fn test_valid_ipv4() {
        assert!(is_valid_ipv4("192.168.1.1"));
        assert!(is_valid_ipv4("0.0.0.0"));
        assert!(is_valid_ipv4("255.255.255.255"));
    }

    #[test]
    fn test_invalid_ipv4() {
        assert!(!is_valid_ipv4("999.999.999.999"));
        assert!(!is_valid_ipv4("not-an-ip"));
        assert!(!is_valid_ipv4("192.168.1"));
    }

    // --- ipv6 ---

    #[test]
    fn test_valid_ipv6() {
        assert!(is_valid_ipv6("::1"));
        assert!(is_valid_ipv6("2001:db8::1"));
        assert!(is_valid_ipv6("fe80::1"));
        assert!(is_valid_ipv6("2001:0db8:85a3:0000:0000:8a2e:0370:7334"));
    }

    #[test]
    fn test_invalid_ipv6() {
        assert!(!is_valid_ipv6("not-ipv6"));
        assert!(!is_valid_ipv6("192.168.1.1"));
    }

    // --- uri ---

    #[test]
    fn test_valid_uris() {
        assert!(is_valid_uri("https://example.com"));
        assert!(is_valid_uri("http://example.com/path?q=1#frag"));
        assert!(is_valid_uri("urn:isbn:0451450523"));
    }

    #[test]
    fn test_invalid_uris() {
        assert!(!is_valid_uri("not a uri"));
        assert!(!is_valid_uri("://missing-scheme"));
    }

    // --- uri-reference ---

    #[test]
    fn test_valid_uri_references() {
        assert!(is_valid_uri_reference("https://example.com"));
        assert!(is_valid_uri_reference("/path/to/resource"));
        assert!(is_valid_uri_reference("#fragment"));
        assert!(is_valid_uri_reference("relative/path"));
    }

    #[test]
    fn test_invalid_uri_references() {
        assert!(!is_valid_uri_reference(""));
        assert!(!is_valid_uri_reference("has space"));
    }

    // --- uuid ---

    #[test]
    fn test_valid_uuids() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_uuid("550E8400-E29B-41D4-A716-446655440000"));
    }

    #[test]
    fn test_invalid_uuids() {
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716"));
        assert!(!is_valid_uuid("550e8400e29b41d4a716446655440000"));
    }

    // --- uri-template ---

    #[test]
    fn test_valid_uri_templates() {
        assert!(is_valid_uri_template("https://example.com/{id}"));
        assert!(is_valid_uri_template("/path/{var}/end"));
        assert!(is_valid_uri_template("no-braces"));
    }

    #[test]
    fn test_invalid_uri_templates() {
        assert!(!is_valid_uri_template("https://example.com/{id"));
        assert!(!is_valid_uri_template("https://example.com/id}"));
        assert!(!is_valid_uri_template("https://example.com/{{nested}}"));
    }

    // --- json-pointer ---

    #[test]
    fn test_valid_json_pointers() {
        assert!(is_valid_json_pointer(""));
        assert!(is_valid_json_pointer("/foo"));
        assert!(is_valid_json_pointer("/foo/bar"));
        assert!(is_valid_json_pointer("/foo/0"));
        assert!(is_valid_json_pointer("/~0"));
        assert!(is_valid_json_pointer("/~1"));
    }

    #[test]
    fn test_invalid_json_pointers() {
        assert!(!is_valid_json_pointer("no-leading-slash"));
        assert!(!is_valid_json_pointer("/foo/~"));
        assert!(!is_valid_json_pointer("/foo/~2"));
    }

    // --- relative-json-pointer ---

    #[test]
    fn test_valid_relative_json_pointers() {
        assert!(is_valid_relative_json_pointer("0"));
        assert!(is_valid_relative_json_pointer("1/foo"));
        assert!(is_valid_relative_json_pointer("0#"));
        assert!(is_valid_relative_json_pointer("2/foo/bar"));
    }

    #[test]
    fn test_invalid_relative_json_pointers() {
        assert!(!is_valid_relative_json_pointer(""));
        assert!(!is_valid_relative_json_pointer("/foo"));
        assert!(!is_valid_relative_json_pointer("01/foo"));
    }

    // --- regex ---

    #[test]
    fn test_valid_regexes() {
        assert!(is_valid_regex("^[a-z]+$"));
        assert!(is_valid_regex(".*"));
        assert!(is_valid_regex(r"\d{3}-\d{4}"));
    }

    #[test]
    fn test_invalid_regexes() {
        assert!(!is_valid_regex("[invalid"));
        assert!(!is_valid_regex("(unclosed"));
    }

    // --- validate_format integration ---

    #[test]
    fn test_validate_format_returns_none_on_valid() {
        assert!(validate_format(&StringFormat::Date, "2024-01-15").is_none());
        assert!(validate_format(&StringFormat::Email, "user@example.com").is_none());
    }

    #[test]
    fn test_validate_format_returns_error_on_invalid() {
        let err = validate_format(&StringFormat::Date, "not-a-date");
        assert!(err.is_some());
        assert!(err.as_ref().is_some_and(|e| e.contains("date")));
    }

    #[test]
    fn test_validate_format_unknown_always_passes() {
        let fmt = StringFormat::Unknown("my-custom-format".to_string());
        assert!(validate_format(&fmt, "anything goes").is_none());
    }
}
