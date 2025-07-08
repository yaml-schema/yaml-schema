use crate::loader::FromSaphyrMapping;
use crate::utils::format_marker;
use crate::validation::Context;
use crate::validation::Validator;
use crate::Result;
use crate::{loader, Number};
use saphyr::{MarkedYaml, Scalar, YamlData};

/// A number schema
#[derive(Debug, Default, PartialEq)]
pub struct IntegerSchema {
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

impl TryFrom<&MarkedYaml<'_>> for IntegerSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<IntegerSchema> {
        if let YamlData::Mapping(mapping) = &value.data {
            let mut integer_schema = IntegerSchema::default();
            for (key, value) in mapping.iter() {
                if let YamlData::Value(Scalar::String(key)) = &key.data {
                    match key.as_ref() {
                        "minimum" => {
                            integer_schema.minimum = Some(value.try_into()?);
                        }
                        "maximum" => {
                            integer_schema.maximum = Some(value.try_into()?);
                        }
                        "exclusiveMinimum" => {
                            integer_schema.exclusive_minimum = Some(value.try_into()?);
                        }
                        "exclusiveMaximum" => {
                            integer_schema.exclusive_maximum = Some(value.try_into()?);
                        }
                        "multipleOf" => {
                            integer_schema.multiple_of = Some(value.try_into()?);
                        }
                        "type" => {
                            if let YamlData::Value(Scalar::String(s)) = &value.data {
                                if s != "integer" {
                                    return Err(unsupported_type!(
                                        "{} Expected type: integer, but got: {}",
                                        format_marker(&value.span.start),
                                        s
                                    ));
                                }
                            } else {
                                return Err(generic_error!(
                                    "{} Expected string value for `type:`, got {:?}",
                                    format_marker(&value.span.start),
                                    value
                                ));
                            }
                        }
                        _ => unimplemented!("Unsupported key for type: integer: {}", key),
                    }
                } else {
                    return Err(generic_error!(
                        "{} Expected string key, got {:?}",
                        format_marker(&key.span.start),
                        key
                    ));
                }
            }
            Ok(integer_schema)
        } else {
            Err(generic_error!(
                "{} Expected mapping, got {:?}",
                format_marker(&value.span.start),
                value
            ))
        }
    }
}

impl FromSaphyrMapping<IntegerSchema> for IntegerSchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> Result<IntegerSchema> {
        let mut integer_schema = IntegerSchema::default();
        for (key, value) in mapping.iter() {
            if let saphyr::Yaml::Value(scalar) = key {
                if let saphyr::Scalar::String(key) = scalar {
                    match key.as_ref() {
                        "minimum" => {
                            integer_schema.minimum = Some(loader::load_number(value)?);
                        }
                        "maximum" => {
                            integer_schema.maximum = Some(loader::load_number(value)?);
                        }
                        "exclusiveMinimum" => {
                            integer_schema.exclusive_minimum = Some(loader::load_number(value)?);
                        }
                        "exclusiveMaximum" => {
                            integer_schema.exclusive_maximum = Some(loader::load_number(value)?);
                        }
                        "multipleOf" => {
                            integer_schema.multiple_of = Some(loader::load_number(value)?);
                        }
                        "type" => {
                            let s = loader::load_string_value(value)?;
                            if s != "integer" {
                                return Err(unsupported_type!(
                                    "Expected type: integer, but got: {}",
                                    s
                                ));
                            }
                        }
                        _ => unimplemented!("Unsupported key for type: integer: {}", key),
                    }
                }
            } else {
                return Err(expected_scalar!(
                    "Expected a scalar value for the key, got: {:#?}",
                    key
                ));
            }
        }
        Ok(integer_schema)
    }
}

impl std::fmt::Display for IntegerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Integer {self:?}")
    }
}

impl Validator for IntegerSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let data = &value.data;
        if let saphyr::YamlData::Value(scalar) = data {
            if let saphyr::Scalar::Integer(i) = scalar {
                crate::validation::validate_integer(
                    context,
                    &self.minimum,
                    &self.maximum,
                    &self.multiple_of,
                    value,
                    *i,
                )
            } else if let saphyr::Scalar::FloatingPoint(o) = scalar {
                let f = o.into_inner();
                if f.fract() == 0.0 {
                    crate::validation::validate_integer(
                        context,
                        &self.minimum,
                        &self.maximum,
                        &self.multiple_of,
                        value,
                        f as i64,
                    )
                } else {
                    context.add_error(value, format!("Expected an integer, but got: {data:?}"));
                }
            } else {
                context.add_error(value, format!("Expected a number, but got: {data:?}"));
            }
        } else {
            context.add_error(value, format!("Expected a scalar value, but got: {data:?}"));
        }
        if !context.errors.borrow().is_empty() {
            fail_fast!(context)
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saphyr::LoadableYamlNode;

    #[test]
    fn test_integer_schema_against_string() {
        let schema = IntegerSchema::default();
        let context = Context::new(true);
        let docs = saphyr::MarkedYaml::load_from_str("foo").unwrap();
        let result = schema.validate(&context, docs.first().unwrap());
        assert!(result.is_err());
        let errors = context.errors.borrow();
        assert!(!errors.is_empty());
        let first_error = errors.first().unwrap();
        assert_eq!(
            first_error.error,
            "Expected a number, but got: Value(String(\"foo\"))"
        );
    }
}
