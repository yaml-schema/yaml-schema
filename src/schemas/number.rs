use crate::loader::FromSaphyrMapping;
use crate::utils::format_marker;
use crate::validation::Context;
use crate::validation::Validator;
use crate::Result;
use crate::{loader, Number};
use saphyr::{MarkedYaml, Scalar, YamlData};
use std::cmp::Ordering;

/// A number schema
#[derive(Debug, Default, PartialEq)]
pub struct NumberSchema {
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

impl std::fmt::Display for NumberSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Number {self:?}")
    }
}

impl Validator for NumberSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let data = &value.data;
        if let YamlData::Value(scalar) = data {
            if let Scalar::Integer(i) = scalar {
                self.validate_number_i64(context, value, *i)
            } else if let Scalar::FloatingPoint(o) = scalar {
                self.validate_number_f64(context, value, o.into_inner())
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

impl NumberSchema {
    // TODO: This duplicates IntegerSchema::validate_integer(), so, find a neat way to dedupe this
    fn validate_number_i64(&self, context: &Context, value: &MarkedYaml, i: i64) {
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
    }

    fn validate_number_f64(&self, context: &Context, value: &saphyr::MarkedYaml, f: f64) {
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
    }
}

impl TryFrom<&MarkedYaml<'_>> for NumberSchema {
    type Error = crate::Error;
    fn try_from(value: &MarkedYaml) -> Result<NumberSchema> {
        if let YamlData::Mapping(mapping) = &value.data {
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
                        "type" => {
                            if let YamlData::Value(Scalar::String(s)) = &value.data {
                                if s != "number" {
                                    return Err(unsupported_type!(
                                        "{} Expected type: number, but got: {}",
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
                        _ => unimplemented!(),
                    }
                } else {
                    return Err(generic_error!(
                        "{} Expected string key, got {:?}",
                        format_marker(&key.span.start),
                        key
                    ));
                }
            }
            Ok(number_schema)
        } else {
            Err(generic_error!(
                "{} Expected mapping, got {:?}",
                format_marker(&value.span.start),
                value
            ))
        }
    }
}

impl FromSaphyrMapping<NumberSchema> for NumberSchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> Result<NumberSchema> {
        let mut number_schema = NumberSchema::default();
        for (key, value) in mapping.iter() {
            if let Ok(key) = loader::load_string_value(key) {
                match key.as_str() {
                    "minimum" => {
                        let minimum = loader::load_number(value).map_err(|_| {
                            crate::Error::UnsupportedType(format!(
                                "Expected type: integer or float, but got: {:?}",
                                &value
                            ))
                        })?;
                        number_schema.minimum = Some(minimum);
                    }
                    "maximum" => {
                        number_schema.maximum = Some(loader::load_number(value)?);
                    }
                    "exclusiveMinimum" => {
                        number_schema.exclusive_minimum = Some(loader::load_number(value)?);
                    }
                    "exclusiveMaximum" => {
                        number_schema.exclusive_maximum = Some(loader::load_number(value)?);
                    }
                    "multipleOf" => {
                        number_schema.multiple_of = Some(loader::load_number(value)?);
                    }
                    "type" => {
                        let s = loader::load_string_value(value)?;
                        if s != "number" {
                            return Err(unsupported_type!("Expected type: number, but got: {}", s));
                        }
                    }
                    _ => unimplemented!(),
                }
            }
        }
        Ok(number_schema)
    }
}
