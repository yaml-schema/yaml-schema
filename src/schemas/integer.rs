use std::cmp::Ordering;

use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::Number;
use crate::Result;
use crate::utils::format_marker;
use crate::validation::Context;
use crate::validation::Validator;

/// An integer schema
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
            Ok(IntegerSchema::try_from(mapping)?)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for IntegerSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
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
        // TODO: add enum validation
        let enum_values = None;
        if let saphyr::YamlData::Value(scalar) = data {
            if let saphyr::Scalar::Integer(i) = scalar {
                self.validate_integer(context, &enum_values, value, *i);
            } else if let saphyr::Scalar::FloatingPoint(o) = scalar {
                let f = o.into_inner();
                if f.fract() == 0.0 {
                    self.validate_integer(context, &enum_values, value, f as i64);
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

impl IntegerSchema {
    fn validate_integer(
        &self,
        context: &Context,
        enum_values: &Option<Vec<i64>>,
        value: &MarkedYaml,
        i: i64,
    ) {
        if let Some(exclusive_min) = self.exclusive_minimum {
            match exclusive_min {
                Number::Integer(exclusive_min) => {
                    if i <= exclusive_min {
                        context.add_error(
                            value,
                            format!("Number must be greater than {exclusive_min}"),
                        );
                    }
                }
                Number::Float(exclusive_min) => {
                    if (i as f64).partial_cmp(&exclusive_min) != Some(Ordering::Greater) {
                        context.add_error(
                            value,
                            format!("Number must be greater than {exclusive_min}"),
                        );
                    }
                }
            }
        } else if let Some(minimum) = self.minimum {
            match minimum {
                Number::Integer(min) => {
                    if i < min {
                        context.add_error(
                            value,
                            format!("Number must be greater than or equal to {min}"),
                        );
                    }
                }
                Number::Float(min) => {
                    let cmp = (i as f64).partial_cmp(&min);
                    if cmp != Some(Ordering::Less) && cmp != Some(Ordering::Equal) {
                        context.add_error(
                            value,
                            format!("Number must be greater than or equal to {min}"),
                        );
                    }
                }
            }
        }

        if let Some(exclusive_max) = self.exclusive_maximum {
            match exclusive_max {
                Number::Integer(exclusive_max) => {
                    if i >= exclusive_max {
                        context
                            .add_error(value, format!("Number must be less than {exclusive_max}"));
                    }
                }
                Number::Float(exclusive_max) => {
                    if (i as f64).partial_cmp(&exclusive_max) != Some(Ordering::Less) {
                        context
                            .add_error(value, format!("Number must be less than {exclusive_max}"));
                    }
                }
            }
        } else if let Some(maximum) = self.maximum {
            match maximum {
                Number::Integer(max) => {
                    if i > max {
                        context.add_error(
                            value,
                            format!("Number must be less than or equal to {max}"),
                        );
                    }
                }
                Number::Float(max) => {
                    let cmp = (i as f64).partial_cmp(&max);
                    if cmp != Some(Ordering::Greater) && cmp != Some(Ordering::Equal) {
                        context.add_error(
                            value,
                            format!("Number must be less than or equal to {max}"),
                        );
                    }
                }
            }
        }

        if let Some(multiple_of) = self.multiple_of {
            match multiple_of {
                Number::Integer(multiple) => {
                    if i % multiple != 0 {
                        context
                            .add_error(value, format!("Number is not a multiple of {multiple}!"));
                    }
                }
                Number::Float(multiple) => {
                    if (i as f64) % multiple != 0.0 {
                        context
                            .add_error(value, format!("Number is not a multiple of {multiple}!"));
                    }
                }
            }
        }
        if let Some(enum_values) = enum_values
            && !enum_values.contains(&i)
        {
            context.add_error(value, format!("Number is not in enum: {enum_values:?}"));
        }
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
