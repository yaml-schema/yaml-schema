use std::cmp::Ordering;
use std::collections::HashMap;

use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::Number;
use crate::Result;
use crate::utils::format_hash_map;
use crate::utils::format_marker;
use crate::validation::Context;
use crate::validation::Validator;

/// A number schema
#[derive(Default, PartialEq)]
pub struct NumberSchema {
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

impl Validator for NumberSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[NumberSchema#validate] self: {self:?}");
        let data = &value.data;
        debug!("[NumberSchema#validate] data: {data:?}");
        if let YamlData::Value(scalar) = data {
            if let Scalar::Integer(i) = scalar {
                // TODO: add enum validation
                let enum_values = None;
                self.validate_number_i64(context, &enum_values, value, *i)
            } else if let Scalar::FloatingPoint(ordered_float) = scalar {
                // TODO: add enum validation
                let enum_values = None;
                self.validate_number_f64(context, &enum_values, value, ordered_float.into_inner())
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

impl NumberSchema {
    // TODO: This duplicates IntegerSchema::validate_integer(), so, find a neat way to dedupe this
    fn validate_number_i64(
        &self,
        context: &Context,
        enum_values: &Option<Vec<i64>>,
        value: &MarkedYaml,
        i: i64,
    ) {
        debug!("[NumberSchema#validate_number_i64] self: {self:?}");
        debug!("[NumberSchema#validate_number_i64] enum_values: {enum_values:?}");
        debug!(
            "[NumberSchema#validate_number_i64] value: {:?}",
            &value.data
        );
        debug!("[NumberSchema#validate_number_i64] i: {i}");
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
                    let cmp = min.partial_cmp(&(i as f64));
                    if cmp == Some(Ordering::Less) {
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

    fn validate_number_f64(
        &self,
        context: &Context,
        enum_values: &Option<Vec<f64>>,
        value: &MarkedYaml,
        f: f64,
    ) {
        if let Some(minimum) = &self.minimum {
            match minimum {
                Number::Integer(min) => {
                    if f < *min as f64 {
                        context.add_error(value, "Number is too small!".to_string());
                    }
                }
                Number::Float(min) => {
                    if f < *min {
                        context.add_error(value, "Number is too small!".to_string());
                    }
                }
            }
        }
        if let Some(maximum) = &self.maximum {
            match maximum {
                Number::Integer(max) => {
                    if f > *max as f64 {
                        context.add_error(value, "Number is too big!".to_string());
                    }
                }
                Number::Float(max) => {
                    if f > *max {
                        context.add_error(value, "Number is too big!".to_string());
                    }
                }
            }
        }
        if let Some(enum_values) = enum_values
            && !enum_values.contains(&f)
        {
            context.add_error(value, format!("Number is not in enum: {enum_values:?}"));
        }
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
        let mut number_schema = NumberSchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(key)) = &key.data {
                match key.as_ref() {
                    "minimum" => {
                        number_schema.minimum = Some(value.try_into()?);
                    }
                    "maximum" => {
                        number_schema.maximum = Some(value.try_into()?);
                    }
                    "exclusiveMinimum" => {
                        number_schema.exclusive_minimum = Some(value.try_into()?);
                    }
                    "exclusiveMaximum" => {
                        number_schema.exclusive_maximum = Some(value.try_into()?);
                    }
                    "multipleOf" => {
                        number_schema.multiple_of = Some(value.try_into()?);
                    }
                    // Maybe this should be handled by the base schema?
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
                        return Err(schema_loading_error!(
                            "Unsupported key for type: number: {}",
                            key
                        ));
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
        Ok(number_schema)
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
        if let Some(minimum) = self.minimum {
            h.insert("minimum".to_string(), minimum.to_string());
        }
        if let Some(maximum) = self.maximum {
            h.insert("maximum".to_string(), maximum.to_string());
        }
        if let Some(exclusive_minimum) = self.exclusive_minimum {
            h.insert(
                "exclusiveMinimum".to_string(),
                exclusive_minimum.to_string(),
            );
        }
        if let Some(exclusive_maximum) = self.exclusive_maximum {
            h.insert(
                "exclusiveMaximum".to_string(),
                exclusive_maximum.to_string(),
            );
        }
        if let Some(multiple_of) = self.multiple_of {
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
            minimum: Some(Number::Integer(1)),
            ..Default::default()
        };
        println!("number_schema: {number_schema:?}");
        let marked_yaml = MarkedYaml::value_from_str("1");
        println!("marked_yaml: {marked_yaml:?}");
        let context = Context::default();
        number_schema
            .validate(&context, &marked_yaml)
            .expect("validate() failed!");
        println!("context: {context:?}");
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
        println!("context: {context:?}");
        assert!(context.has_errors());
    }
}
