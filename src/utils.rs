// Various utility functions
use crate::Result;
use hashlink::linked_hash_map;
use saphyr::{MarkedYaml, Scalar, YamlData};
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

/// Create and return a HashMap with a single key & value
pub fn hash_map<K, V>(key: K, value: V) -> HashMap<K, V>
where
    K: Hash + Eq + Clone,
{
    let mut hash_map = HashMap::with_capacity(1);
    hash_map.insert(key, value);
    hash_map
}

/// Create and return a LinkedHashMap with a single key & value
pub fn linked_hash_map<K, V>(key: K, value: V) -> linked_hash_map::LinkedHashMap<K, V>
where
    K: Hash + Eq + Clone,
{
    let mut linked_hash_map = linked_hash_map::LinkedHashMap::new();
    linked_hash_map.insert(key, value);
    linked_hash_map
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

/// Formats a saphyr::Marker as a string. Displays the line and column as a pair of numbers, separated by a comma.
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

/// Formats a HashMap as a string, ala JSON
pub fn format_hash_map<K, V>(hash_map: &HashMap<K, V>) -> String
where
    K: std::fmt::Display,
    V: std::fmt::Display,
{
    let items: Vec<String> = hash_map
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect();
    format!("{{ {} }}", items.join(", "))
}
/// Collects the keys of a list of SchemaMetadata implementations into a single slice of strings.
pub fn collect_keys(a: &'static [&'static str], b: &'static [&'static str]) -> Vec<&'static str> {
    let mut keys = Vec::with_capacity(a.len() + b.len());
    keys.extend_from_slice(a);
    keys.extend_from_slice(b);
    keys.sort();
    keys.dedup();
    keys
}

/// Filters a saphyr::Mapping and returns a new mapping with only the keys that are in the list.
pub fn filter_mapping<'a>(
    mapping: &saphyr::AnnotatedMapping<'a, saphyr::MarkedYaml<'a>>,
    keys: Vec<&'static str>,
    override_type: &'a str,
) -> Result<saphyr::AnnotatedMapping<'a, saphyr::MarkedYaml<'a>>> {
    let mut filtered_mapping = saphyr::AnnotatedMapping::new();
    for (k, v) in mapping.iter() {
        if let YamlData::Value(Scalar::String(key)) = &k.data {
            if keys.contains(&key.as_ref()) {
                match key.as_ref() {
                    "type" => {
                        filtered_mapping
                            .insert(k.clone(), MarkedYaml::value_from_str(override_type));
                    }
                    _ => {
                        filtered_mapping.insert(k.clone(), v.clone());
                    }
                }
            }
        } else {
            return Err(expected_scalar!("Expected a string key, got: {:?}", k.data));
        }
    }
    Ok(filtered_mapping.into_iter().collect())
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
