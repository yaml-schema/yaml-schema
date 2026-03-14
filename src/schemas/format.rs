use std::fmt;
use std::str::FromStr;

/// Represents a JSON Schema `format` value for string validation.
///
/// Known formats are validated; unknown formats are annotation-only
/// (they always pass validation).
#[derive(Clone, PartialEq, Eq)]
pub enum StringFormat {
    DateTime,
    Date,
    Time,
    Duration,
    Email,
    IdnEmail,
    Hostname,
    IdnHostname,
    Ipv4,
    Ipv6,
    Uri,
    UriReference,
    Iri,
    IriReference,
    Uuid,
    UriTemplate,
    JsonPointer,
    RelativeJsonPointer,
    Regex,
    Unknown(String),
}

impl FromStr for StringFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "date-time" => StringFormat::DateTime,
            "date" => StringFormat::Date,
            "time" => StringFormat::Time,
            "duration" => StringFormat::Duration,
            "email" => StringFormat::Email,
            "idn-email" => StringFormat::IdnEmail,
            "hostname" => StringFormat::Hostname,
            "idn-hostname" => StringFormat::IdnHostname,
            "ipv4" => StringFormat::Ipv4,
            "ipv6" => StringFormat::Ipv6,
            "uri" => StringFormat::Uri,
            "uri-reference" => StringFormat::UriReference,
            "iri" => StringFormat::Iri,
            "iri-reference" => StringFormat::IriReference,
            "uuid" => StringFormat::Uuid,
            "uri-template" => StringFormat::UriTemplate,
            "json-pointer" => StringFormat::JsonPointer,
            "relative-json-pointer" => StringFormat::RelativeJsonPointer,
            "regex" => StringFormat::Regex,
            other => StringFormat::Unknown(other.to_string()),
        })
    }
}

impl fmt::Display for StringFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringFormat::DateTime => write!(f, "date-time"),
            StringFormat::Date => write!(f, "date"),
            StringFormat::Time => write!(f, "time"),
            StringFormat::Duration => write!(f, "duration"),
            StringFormat::Email => write!(f, "email"),
            StringFormat::IdnEmail => write!(f, "idn-email"),
            StringFormat::Hostname => write!(f, "hostname"),
            StringFormat::IdnHostname => write!(f, "idn-hostname"),
            StringFormat::Ipv4 => write!(f, "ipv4"),
            StringFormat::Ipv6 => write!(f, "ipv6"),
            StringFormat::Uri => write!(f, "uri"),
            StringFormat::UriReference => write!(f, "uri-reference"),
            StringFormat::Iri => write!(f, "iri"),
            StringFormat::IriReference => write!(f, "iri-reference"),
            StringFormat::Uuid => write!(f, "uuid"),
            StringFormat::UriTemplate => write!(f, "uri-template"),
            StringFormat::JsonPointer => write!(f, "json-pointer"),
            StringFormat::RelativeJsonPointer => write!(f, "relative-json-pointer"),
            StringFormat::Regex => write!(f, "regex"),
            StringFormat::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl fmt::Debug for StringFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringFormat::Unknown(s) => write!(f, "StringFormat::Unknown({s:?})"),
            _ => write!(f, "StringFormat::{}", capitalize_variant(self)),
        }
    }
}

fn capitalize_variant(format: &StringFormat) -> &'static str {
    match format {
        StringFormat::DateTime => "DateTime",
        StringFormat::Date => "Date",
        StringFormat::Time => "Time",
        StringFormat::Duration => "Duration",
        StringFormat::Email => "Email",
        StringFormat::IdnEmail => "IdnEmail",
        StringFormat::Hostname => "Hostname",
        StringFormat::IdnHostname => "IdnHostname",
        StringFormat::Ipv4 => "Ipv4",
        StringFormat::Ipv6 => "Ipv6",
        StringFormat::Uri => "Uri",
        StringFormat::UriReference => "UriReference",
        StringFormat::Iri => "Iri",
        StringFormat::IriReference => "IriReference",
        StringFormat::Uuid => "Uuid",
        StringFormat::UriTemplate => "UriTemplate",
        StringFormat::JsonPointer => "JsonPointer",
        StringFormat::RelativeJsonPointer => "RelativeJsonPointer",
        StringFormat::Regex => "Regex",
        StringFormat::Unknown(_) => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_known_formats() {
        let cases = [
            ("date-time", StringFormat::DateTime),
            ("date", StringFormat::Date),
            ("time", StringFormat::Time),
            ("duration", StringFormat::Duration),
            ("email", StringFormat::Email),
            ("idn-email", StringFormat::IdnEmail),
            ("hostname", StringFormat::Hostname),
            ("idn-hostname", StringFormat::IdnHostname),
            ("ipv4", StringFormat::Ipv4),
            ("ipv6", StringFormat::Ipv6),
            ("uri", StringFormat::Uri),
            ("uri-reference", StringFormat::UriReference),
            ("iri", StringFormat::Iri),
            ("iri-reference", StringFormat::IriReference),
            ("uuid", StringFormat::Uuid),
            ("uri-template", StringFormat::UriTemplate),
            ("json-pointer", StringFormat::JsonPointer),
            ("relative-json-pointer", StringFormat::RelativeJsonPointer),
            ("regex", StringFormat::Regex),
        ];
        for (input, expected) in cases {
            let parsed: StringFormat = input.parse().unwrap();
            assert_eq!(parsed, expected, "Failed for input: {input}");
        }
    }

    #[test]
    fn test_from_str_unknown_format() {
        let parsed: StringFormat = "my-custom-format".parse().unwrap();
        assert_eq!(
            parsed,
            StringFormat::Unknown("my-custom-format".to_string())
        );
    }

    #[test]
    fn test_display_roundtrip() {
        let cases = [
            "date-time",
            "date",
            "time",
            "duration",
            "email",
            "idn-email",
            "hostname",
            "idn-hostname",
            "ipv4",
            "ipv6",
            "uri",
            "uri-reference",
            "iri",
            "iri-reference",
            "uuid",
            "uri-template",
            "json-pointer",
            "relative-json-pointer",
            "regex",
        ];
        for input in cases {
            let parsed: StringFormat = input.parse().unwrap();
            assert_eq!(parsed.to_string(), input);
        }
    }

    #[test]
    fn test_debug_format() {
        assert_eq!(
            format!("{:?}", StringFormat::DateTime),
            "StringFormat::DateTime"
        );
        assert_eq!(
            format!("{:?}", StringFormat::Unknown("custom".to_string())),
            r#"StringFormat::Unknown("custom")"#
        );
    }
}
