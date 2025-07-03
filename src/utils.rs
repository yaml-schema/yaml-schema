// Various utility functions
use std::borrow::Cow;

use crate::Result;

/// Construct a saphyr::Yaml scalar value from a &str
pub const fn saphyr_yaml_string(s: &str) -> saphyr::Yaml<'_> {
    saphyr::Yaml::Value(saphyr::Scalar::String(Cow::Borrowed(s)))
}

/// Try to unwrap a saphyr::Scalar from a saphyr::Yaml
pub fn try_unwrap_saphyr_scalar<'a>(yaml: &'a saphyr::Yaml) -> Result<&'a saphyr::Scalar<'a>> {
    if let saphyr::Yaml::Value(scalar) = yaml {
        Ok(scalar)
    } else {
        Err(expected_scalar!("Expected a scalar, got: {:?}", yaml))
    }
}

/// Formats a saphyr::Scalar as a string
pub fn format_scalar(scalar: &saphyr::Scalar) -> String {
    match scalar {
        saphyr::Scalar::Null => "null".to_string(),
        saphyr::Scalar::Boolean(b) => b.to_string(),
        saphyr::Scalar::Integer(i) => i.to_string(),
        saphyr::Scalar::FloatingPoint(o) => o.to_string(),
        saphyr::Scalar::String(s) => format!("\"{s}\""),
    }
}

/// Formats a saphyr::YamlData as a string
pub fn format_yaml_data<'a>(data: &saphyr::YamlData<'a, saphyr::MarkedYaml<'a>>) -> String {
    match data {
        saphyr::YamlData::Value(scalar) => format_scalar(scalar),
        saphyr::YamlData::Sequence(seq) => {
            let items: Vec<String> = seq.iter().map(|v| format_yaml_data(&v.data)).collect();
            format!("[{}]", items.join(", "))
        }
        saphyr::YamlData::Mapping(mapping) => {
            let items: Vec<String> = mapping
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}: {}",
                        format_yaml_data(&k.data),
                        format_yaml_data(&v.data)
                    )
                })
                .collect();
            format!("[{}]", items.join(", "))
        }
        _ => format!("<unsupported type: {data:?}>"),
    }
}

pub fn format_marker(marker: &saphyr::Marker) -> String {
    format!("[{}, {}]", marker.line(), marker.col())
}

/// Formats a vector of values as a string, by joining them with commas
pub fn format_vec<V>(vec: &[V]) -> String
where
    V: std::fmt::Display,
{
    let items: Vec<String> = vec.iter().map(|v| format!("{v}")).collect();
    format!("[{}]", items.join(", "))
}
