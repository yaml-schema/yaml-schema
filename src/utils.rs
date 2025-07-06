// Various utility functions
use crate::Result;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

/// Create a return a HashMap with a single key & value
pub fn hash_map<K, V>(key: K, value: V) -> HashMap<K, V>
where
    K: Hash + Eq + Clone,
{
    let mut hash_map = HashMap::with_capacity(1);
    hash_map.insert(key, value);
    hash_map
}

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

/// Converts a saphyr::Scalar value to a String. Does NOT enclose Scalar::String values in
/// double-quotes.
pub fn scalar_to_string(scalar: &saphyr::Scalar) -> String {
    match scalar {
        saphyr::Scalar::Null => "null".to_string(),
        saphyr::Scalar::Boolean(b) => b.to_string(),
        saphyr::Scalar::Integer(i) => i.to_string(),
        saphyr::Scalar::FloatingPoint(o) => o.to_string(),
        saphyr::Scalar::String(s) => s.to_string(),
    }
}

/// Formats a saphyr::Scalar as a string. Encloses Scalar::String values in double quotes (`"`)
pub fn format_scalar(scalar: &saphyr::Scalar) -> String {
    match scalar {
        saphyr::Scalar::String(s) => format!("\"{s}\""),
        _ => scalar_to_string(scalar),
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

#[cfg(test)]
mod tests {
    use crate::utils::{format_scalar, hash_map, scalar_to_string};
    use ordered_float::OrderedFloat;
    use std::collections::HashMap;

    #[test]
    fn test_hash_map() {
        let expected = vec![("foo".to_string(), "bar".to_string())]
            .into_iter()
            .collect::<HashMap<String, String>>();

        let actual = hash_map("foo".to_string(), "bar".to_string());
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_scalar_to_string() {
        assert_eq!("null", scalar_to_string(&saphyr::Scalar::Null));
        assert_eq!("true", scalar_to_string(&saphyr::Scalar::Boolean(true)));
        assert_eq!("false", scalar_to_string(&saphyr::Scalar::Boolean(false)));
        assert_eq!("42", scalar_to_string(&saphyr::Scalar::Integer(42)));
        assert_eq!("-1", scalar_to_string(&saphyr::Scalar::Integer(-1)));
        assert_eq!(
            "3.14",
            scalar_to_string(&saphyr::Scalar::FloatingPoint(OrderedFloat::from(3.14)))
        );
        assert_eq!(
            "foo",
            scalar_to_string(&saphyr::Scalar::String("foo".into()))
        );
    }

    #[test]
    fn test_format_scalar() {
        assert_eq!("null", format_scalar(&saphyr::Scalar::Null));
        assert_eq!("true", format_scalar(&saphyr::Scalar::Boolean(true)));
        assert_eq!("false", format_scalar(&saphyr::Scalar::Boolean(false)));
        assert_eq!("42", format_scalar(&saphyr::Scalar::Integer(42)));
        assert_eq!("-1", format_scalar(&saphyr::Scalar::Integer(-1)));
        assert_eq!(
            "3.14",
            format_scalar(&saphyr::Scalar::FloatingPoint(OrderedFloat::from(3.14)))
        );
        assert_eq!(
            "\"foo\"",
            format_scalar(&saphyr::Scalar::String("foo".into()))
        );
    }
}
