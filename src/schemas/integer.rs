use crate::ConstValue;
use crate::Result;
use crate::loader::FromSaphyrMapping;
use crate::schemas::BaseSchema;
use crate::utils::format_marker;
use crate::validation::Context;
use crate::validation::Validator;
use crate::{Number, loader};
use saphyr::{MarkedYaml, Scalar, YamlData};
use std::cmp::Ordering;

/// An integer schema
#[derive(Debug, PartialEq)]
pub struct IntegerSchema {
    pub base: BaseSchema,
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

impl Default for IntegerSchema {
    fn default() -> Self {
        Self {
            base: BaseSchema::type_integer(),
            minimum: None,
            maximum: None,
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
        }
    }
}

impl TryFrom<&MarkedYaml<'_>> for IntegerSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<IntegerSchema> {
        if let YamlData::Mapping(mapping) = &value.data {
            let mut integer_schema = IntegerSchema::from_base(BaseSchema::try_from(value)?);
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
                        // These should've been handled by the base schema
                        "type" => (),
                        "const" => (),
                        "enum" => (),
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
            Err(expected_mapping!(value))
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
        let enum_values = self.base.r#enum.as_ref().map(|r#enum| {
            r#enum
                .iter()
                .filter_map(|v| {
                    if let ConstValue::Number(Number::Integer(i)) = v {
                        Some(*i)
                    } else {
                        None
                    }
                })
                .collect::<Vec<i64>>()
        });
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
    pub fn from_base(base: BaseSchema) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

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
                    if i <= min {
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
                        context.add_error(
                            value,
                            format!("Number must be less than than {exclusive_max}"),
                        );
                    }
                }
                Number::Float(exclusive_max) => {
                    if (i as f64).partial_cmp(&exclusive_max) != Some(Ordering::Less) {
                        context.add_error(
                            value,
                            format!("Number must be less than than {exclusive_max}"),
                        );
                    }
                }
            }
        } else if let Some(maximum) = self.maximum {
            match maximum {
                Number::Integer(max) => {
                    if i >= max {
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
