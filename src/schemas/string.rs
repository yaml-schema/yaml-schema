use crate::loader::FromSaphyrMapping;
use crate::schemas::r#enum::load_enum_values;
use crate::utils::format_marker;
use crate::{ConstValue, Schema, YamlSchema, loader};
use regex::Regex;
use saphyr::{MarkedYaml, Scalar, YamlData};

/// A string schema
#[derive(Debug, Default)]
pub struct StringSchema {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<Regex>,
    pub r#enum: Option<Vec<String>>,
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

impl From<StringSchema> for YamlSchema {
    fn from(value: StringSchema) -> Self {
        YamlSchema {
            schema: Some(Schema::String(value)),
            ..Default::default()
        }
    }
}

impl TryFrom<&MarkedYaml<'_>> for StringSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<StringSchema, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
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
                        "type" => {
                            if let YamlData::Value(Scalar::String(s)) = &value.data {
                                if s != "string" {
                                    return Err(unsupported_type!(
                                        "Expected type: string, but got: {}",
                                        s
                                    ));
                                }
                            } else {
                                return Err(generic_error!(
                                    "{} Expected type: string, but got: {:?}",
                                    format_marker(&value.span.start),
                                    value
                                ));
                            }
                        }
                        "enum" => {
                            if let YamlData::Sequence(sequence) = &value.data {
                                let enum_values: Vec<ConstValue> = load_enum_values(sequence)?;
                                let string_enum_values = enum_values
                                    .iter()
                                    .map(|v| match v {
                                        ConstValue::String(s) => Ok(s.clone()),
                                        _ => Ok(format!("{v}")),
                                    })
                                    .collect::<crate::Result<Vec<String>>>()?;
                                string_schema.r#enum = Some(string_enum_values);
                            } else {
                                return Err(unsupported_type!(
                                    "enum expected array, but got: {:?}",
                                    value
                                ));
                            }
                        }
                        _ => unimplemented!("Unsupported key for type: string: {}", key),
                    }
                }
            }
            Ok(string_schema)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl FromSaphyrMapping<StringSchema> for StringSchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> crate::Result<StringSchema> {
        let mut string_schema = StringSchema::default();
        for (key, value) in mapping.iter() {
            if let Ok(key) = loader::load_string_value(key) {
                match key.as_str() {
                    "minLength" => {
                        if let Ok(i) = loader::load_integer(value) {
                            string_schema.min_length = Some(i as usize);
                        } else {
                            return Err(unsupported_type!(
                                "minLength expected integer, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "maxLength" => {
                        if let Ok(i) = loader::load_integer(value) {
                            string_schema.max_length = Some(i as usize);
                        } else {
                            return Err(unsupported_type!(
                                "maxLength expected integer, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "pattern" => {
                        if let Ok(s) = loader::load_string_value(value) {
                            let regex = regex::Regex::new(s.as_str())?;
                            string_schema.pattern = Some(regex);
                        } else {
                            return Err(unsupported_type!(
                                "pattern expected string, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "type" => {
                        let s = loader::load_string_value(value)?;
                        if s != "string" {
                            return Err(unsupported_type!("Expected type: string, but got: {}", s));
                        }
                    }
                    "enum" => {
                        if let saphyr::Yaml::Sequence(sequence) = value {
                            let enum_values: Vec<ConstValue> =
                                sequence.iter().map(ConstValue::from_saphyr_yaml).collect();
                            let string_enum_values = enum_values
                                .iter()
                                .map(|v| match v {
                                    ConstValue::String(s) => Ok(s.clone()),
                                    _ => Ok(format!("{v}")),
                                })
                                .collect::<crate::Result<Vec<String>>>()?;
                            string_schema.r#enum = Some(string_enum_values);
                        } else {
                            return Err(unsupported_type!(
                                "enum expected array, but got: {:?}",
                                value
                            ));
                        }
                    }
                    _ => unimplemented!("Unsupported key for type: string: {}", key),
                }
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

    pub fn r#enum(&mut self, r#enum: Vec<String>) -> &mut Self {
        self.0.r#enum = Some(r#enum);
        self
    }

    pub fn add_enum<S>(&mut self, s: S) -> &mut Self
    where
        S: Into<String>,
    {
        if let Some(r#enum) = self.0.r#enum.as_mut() {
            r#enum.push(s.into());
            self
        } else {
            self.r#enum(vec![s.into()])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_schema_builder() {
        let schema = StringSchema::builder()
            .add_enum("foo")
            .add_enum("bar")
            .build();
        assert_eq!(
            StringSchema {
                min_length: None,
                max_length: None,
                pattern: None,
                r#enum: Some(vec!["foo".into(), "bar".into()]),
            },
            schema
        );
    }
}
