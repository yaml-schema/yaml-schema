use regex::Regex;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::ConstValue;
use crate::Schema;
use crate::YamlSchema;
use crate::loader;
use crate::loader::FromSaphyrMapping;
use crate::schemas::base::BaseSchema;

/// A string schema
#[derive(Debug)]
pub struct StringSchema {
    pub base: BaseSchema,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<Regex>,
}

impl Default for StringSchema {
    fn default() -> Self {
        Self {
            base: BaseSchema::type_string(),
            min_length: None,
            max_length: None,
            pattern: None,
        }
    }
}

impl StringSchema {
    pub fn from_base(base: BaseSchema) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

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
            let mut string_schema = StringSchema::from_base(BaseSchema::try_from(value)?);
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
                        // These should've been handled by the base schema
                        "type" => (),
                        "const" => (),
                        "enum" => (),
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
        self.0.base.r#enum = Some(r#enum.into_iter().map(ConstValue::string).collect());
        self
    }

    pub fn add_enum<S>(&mut self, s: S) -> &mut Self
    where
        S: Into<String>,
    {
        if let Some(r#enum) = self.0.base.r#enum.as_mut() {
            r#enum.push(ConstValue::string(s.into()));
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
                base: BaseSchema {
                    r#enum: Some(vec![ConstValue::string("foo"), ConstValue::string("bar")]),
                    ..Default::default()
                },
                ..Default::default()
            },
            schema
        );
    }
}
