use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::Number;
use crate::Result;
use crate::schemas::NumericBounds;
use crate::utils::format_marker;
use crate::validation::Context;
use crate::validation::Validator;

/// An integer schema
#[derive(Debug, Default, PartialEq)]
pub struct IntegerSchema {
    pub bounds: NumericBounds,
}

impl TryFrom<&MarkedYaml<'_>> for IntegerSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<IntegerSchema> {
        if let YamlData::Mapping(mapping) = &value.data {
            Ok(IntegerSchema::try_from(mapping)?)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for IntegerSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let mut schema = IntegerSchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(key)) = &key.data {
                match key.as_ref() {
                    "minimum" => {
                        schema.bounds.minimum = Some(value.try_into()?);
                    }
                    "maximum" => {
                        schema.bounds.maximum = Some(value.try_into()?);
                    }
                    "exclusiveMinimum" => {
                        schema.bounds.exclusive_minimum = Some(value.try_into()?);
                    }
                    "exclusiveMaximum" => {
                        schema.bounds.exclusive_maximum = Some(value.try_into()?);
                    }
                    "multipleOf" => {
                        schema.bounds.multiple_of = Some(value.try_into()?);
                    }
                    _ => {
                        debug!("Unsupported key for `type: integer`: {}", key);
                    }
                }
            } else {
                return Err(expected_scalar!(
                    "{} Expected string key, got {:?}",
                    format_marker(&key.span.start),
                    key
                ));
            }
        }
        Ok(schema)
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
                self.bounds.validate(context, value, Number::Integer(*i));
            } else if let saphyr::Scalar::FloatingPoint(o) = scalar {
                let f = o.into_inner();
                if f.fract() == 0.0 {
                    self.bounds
                        .validate(context, value, Number::Integer(f as i64));
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
    use saphyr::LoadableYamlNode;

    use crate::YamlSchema;

    use super::*;

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

    #[test]
    fn test_minimum_float_accepts_value_above() {
        let schema = IntegerSchema {
            bounds: NumericBounds {
                minimum: Some(Number::Float(1.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("2");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(!context.has_errors());
    }

    #[test]
    fn test_minimum_float_rejects_value_below() {
        let schema = IntegerSchema {
            bounds: NumericBounds {
                minimum: Some(Number::Float(1.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("1");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_maximum_float_accepts_value_below() {
        let schema = IntegerSchema {
            bounds: NumericBounds {
                maximum: Some(Number::Float(10.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("10");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(!context.has_errors());
    }

    #[test]
    fn test_maximum_float_rejects_value_above() {
        let schema = IntegerSchema {
            bounds: NumericBounds {
                maximum: Some(Number::Float(10.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("11");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_minimum_float_accepts_value_above() {
        let schema = IntegerSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Float(1.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("2");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(!context.has_errors());
    }

    #[test]
    fn test_exclusive_minimum_float_rejects_value_below() {
        let schema = IntegerSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Float(1.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("1");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_maximum_float_accepts_value_below() {
        let schema = IntegerSchema {
            bounds: NumericBounds {
                exclusive_maximum: Some(Number::Float(10.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("10");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(!context.has_errors());
    }

    #[test]
    fn test_exclusive_maximum_float_rejects_value_above() {
        let schema = IntegerSchema {
            bounds: NumericBounds {
                exclusive_maximum: Some(Number::Float(10.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("11");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_integer_schema_with_description() {
        let yaml = r#"
        type: integer
        description: The description
        "#;
        let marked_yaml = MarkedYaml::load_from_str(yaml).unwrap();
        let integer_schema = YamlSchema::try_from(marked_yaml.first().unwrap()).unwrap();
        let YamlSchema::Subschema(subschema) = &integer_schema else {
            panic!("Expected a subschema");
        };
        assert_eq!(
            subschema.metadata_and_annotations.description,
            Some("The description".to_string())
        );
    }
}
