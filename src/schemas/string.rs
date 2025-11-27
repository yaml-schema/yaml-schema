use std::collections::HashMap;

use log::debug;
use regex::Regex;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::loader;
use crate::utils::format_hash_map;
use crate::utils::format_marker;

/// A string schema
#[derive(Default)]
pub struct StringSchema {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<Regex>,
}

impl std::fmt::Debug for StringSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut h = HashMap::new();
        if let Some(min_length) = self.min_length {
            h.insert("minLength".to_string(), min_length.to_string());
        }
        if let Some(max_length) = self.max_length {
            h.insert("maxLength".to_string(), max_length.to_string());
        }
        if let Some(pattern) = &self.pattern {
            h.insert("pattern".to_string(), pattern.as_str().to_string());
        }
        write!(f, "StringSchema {}", format_hash_map(&h))
    }
}

impl StringSchema {
    pub fn builder() -> StringSchemaBuilder {
        StringSchemaBuilder::new()
    }
}

impl PartialEq for StringSchema {
    fn eq(&self, other: &Self) -> bool {
        self.min_length == other.min_length
            && self.max_length == other.max_length
            && are_patterns_equivalent(&self.pattern, &other.pattern)
    }
}

impl TryFrom<&MarkedYaml<'_>> for StringSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<StringSchema, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
            Ok(StringSchema::try_from(mapping)?)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for StringSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let mut string_schema = StringSchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(key)) = &key.data {
                match key.as_ref() {
                    "minLength" => {
                        if let Ok(i) = loader::load_integer_marked(value) {
                            string_schema.min_length = Some(i as usize);
                        } else {
                            return Err(unsupported_type!(
                                "minLength expected integer, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "maxLength" => {
                        if let Ok(i) = loader::load_integer_marked(value) {
                            string_schema.max_length = Some(i as usize);
                        } else {
                            return Err(unsupported_type!(
                                "maxLength expected integer, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "pattern" => {
                        if let YamlData::Value(Scalar::String(s)) = &value.data {
                            let regex = regex::Regex::new(s.as_ref())?;
                            string_schema.pattern = Some(regex);
                        } else {
                            return Err(unsupported_type!(
                                "pattern expected string, but got: {:?}",
                                value
                            ));
                        }
                    }
                    // Maybe this should be handled by the base schema?
                    "type" => {
                        if let YamlData::Value(Scalar::String(s)) = &value.data {
                            if s != "string" {
                                return Err(unsupported_type!(
                                    "Expected type: string, but got: {}",
                                    s
                                ));
                            }
                        } else {
                            return Err(expected_type_is_string!(value));
                        }
                    }
                    _ => {
                        debug!("[StringSchema] Unsupported key for `type: string`: {key}");
                    }
                }
            } else {
                return Err(expected_scalar!(
                    "{} Expected a scalar key, got: {:?}",
                    format_marker(&key.span.start),
                    key
                ));
            }
        }
        Ok(string_schema)
    }
}
/// 'Naive' check to see if two regexes are equal, by comparing their string representations
/// We do it this way because we can't `impl PartialEq for Regex` and don't want to have to
/// alias or wrap the `regex::Regex` type
fn are_patterns_equivalent(a: &Option<Regex>, b: &Option<Regex>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.as_str() == b.as_str(),
        (None, None) => true,
        _ => false,
    }
}

impl std::fmt::Display for StringSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StringSchema {{ min_length: {:?}, max_length: {:?}, pattern: {:?} }}",
            self.min_length, self.max_length, self.pattern
        )
    }
}

pub struct StringSchemaBuilder(StringSchema);

impl Default for StringSchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StringSchemaBuilder {
    pub fn new() -> Self {
        Self(StringSchema::default())
    }

    pub fn build(&mut self) -> StringSchema {
        std::mem::take(&mut self.0)
    }

    pub fn min_length(&mut self, min_length: usize) -> &mut Self {
        self.0.min_length = Some(min_length);
        self
    }

    pub fn max_length(&mut self, max_length: usize) -> &mut Self {
        self.0.max_length = Some(max_length);
        self
    }

    pub fn pattern(&mut self, pattern: Regex) -> &mut Self {
        self.0.pattern = Some(pattern);
        self
    }
}
