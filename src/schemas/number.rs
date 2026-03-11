use std::collections::HashMap;

use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::Number;
use crate::Result;
use crate::schemas::NumericBounds;
use crate::utils::format_hash_map;
use crate::utils::format_marker;
use crate::validation::Context;
use crate::validation::Validator;

/// A number schema
#[derive(Default, PartialEq)]
pub struct NumberSchema {
    pub bounds: NumericBounds,
}

impl Validator for NumberSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[NumberSchema#validate] self: {self:?}");
        let data = &value.data;
        debug!("[NumberSchema#validate] data: {data:?}");
        if let YamlData::Value(scalar) = data {
            if let Scalar::Integer(i) = scalar {
                self.bounds.validate(context, value, Number::Integer(*i));
            } else if let Scalar::FloatingPoint(ordered_float) = scalar {
                self.bounds
                    .validate(context, value, Number::Float(ordered_float.into_inner()));
            } else {
                context.add_error(value, format!("Expected a number, but got: {data:?}"));
            }
        } else {
            context.add_error(value, format!("Expected a scalar value, but got: {data:?}"));
        }
        if context.has_errors() {
            fail_fast!(context)
        }
        Ok(())
    }
}

impl TryFrom<&MarkedYaml<'_>> for NumberSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<NumberSchema> {
        if let YamlData::Mapping(mapping) = &value.data {
            Ok(NumberSchema::try_from(mapping)?)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for NumberSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let mut schema = NumberSchema::default();
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
                    "type" => {
                        if let YamlData::Value(Scalar::String(s)) = &value.data {
                            if s != "number" {
                                return Err(unsupported_type!(
                                    "Expected type: number, but got: {}",
                                    s
                                ));
                            }
                        } else if let YamlData::Sequence(values) = &value.data {
                            if !values
                                .iter()
                                .any(|v| v.data == MarkedYaml::value_from_str("number").data)
                            {
                                return Err(unsupported_type!(
                                    "Expected type: number, but got: {:?}",
                                    value
                                ));
                            }
                        } else {
                            return Err(expected_type_is_string!(value));
                        }
                    }
                    _ => {
                        debug!("Unsupported key for type: number: {}", key);
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

impl std::fmt::Display for NumberSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Number {self:?}")
    }
}

impl std::fmt::Debug for NumberSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut h = HashMap::new();
        if let Some(minimum) = self.bounds.minimum {
            h.insert("minimum".to_string(), minimum.to_string());
        }
        if let Some(maximum) = self.bounds.maximum {
            h.insert("maximum".to_string(), maximum.to_string());
        }
        if let Some(exclusive_minimum) = self.bounds.exclusive_minimum {
            h.insert(
                "exclusiveMinimum".to_string(),
                exclusive_minimum.to_string(),
            );
        }
        if let Some(exclusive_maximum) = self.bounds.exclusive_maximum {
            h.insert(
                "exclusiveMaximum".to_string(),
                exclusive_maximum.to_string(),
            );
        }
        if let Some(multiple_of) = self.bounds.multiple_of {
            h.insert("multipleOf".to_string(), multiple_of.to_string());
        }
        write!(f, "Number {}", format_hash_map(&h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_schema_debug() {
        let number_schema = NumberSchema {
            bounds: NumericBounds {
                minimum: Some(Number::Integer(1)),
                ..Default::default()
            },
        };
        let marked_yaml = MarkedYaml::value_from_str("1");
        let context = Context::default();
        number_schema
            .validate(&context, &marked_yaml)
            .expect("validate() failed!");
        assert!(!context.has_errors());
    }

    #[test]
    fn test_number_schema_should_not_accept_boolean() {
        let number_schema = NumberSchema::default();
        let marked_yaml = MarkedYaml::value_from_str("true");
        assert!(marked_yaml.data.is_boolean());
        let context = Context::default();
        number_schema
            .validate(&context, &marked_yaml)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_minimum_float_accepts_value_above() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Float(1.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("1.6");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(!context.has_errors());
    }

    #[test]
    fn test_exclusive_minimum_float_rejects_equal_value() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Float(1.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("1.5");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_minimum_float_rejects_value_below() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Float(1.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("1.4");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_maximum_float_accepts_value_below() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_maximum: Some(Number::Float(10.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("10.4");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(!context.has_errors());
    }

    #[test]
    fn test_exclusive_maximum_float_rejects_equal_value() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_maximum: Some(Number::Float(10.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("10.5");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_maximum_float_rejects_value_above() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_maximum: Some(Number::Float(10.5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("10.6");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_minimum_int_boundary_with_float_value() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Integer(5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("5.0");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_maximum_int_boundary_with_float_value() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_maximum: Some(Number::Integer(5)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("5.0");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_min_and_max_float_accepts_value_in_range() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Float(1.0)),
                exclusive_maximum: Some(Number::Float(10.0)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("5.5");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(!context.has_errors());
    }

    #[test]
    fn test_exclusive_min_and_max_float_rejects_lower_boundary() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Float(1.0)),
                exclusive_maximum: Some(Number::Float(10.0)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("1.0");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }

    #[test]
    fn test_exclusive_min_and_max_float_rejects_upper_boundary() {
        let schema = NumberSchema {
            bounds: NumericBounds {
                exclusive_minimum: Some(Number::Float(1.0)),
                exclusive_maximum: Some(Number::Float(10.0)),
                ..Default::default()
            },
        };
        let value = MarkedYaml::value_from_str("10.0");
        let context = Context::default();
        schema
            .validate(&context, &value)
            .expect("validate() failed!");
        assert!(context.has_errors());
    }
}
