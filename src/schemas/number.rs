use std::cmp::Ordering;

use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::ConstValue;
use crate::Number;
use crate::Result;
use crate::schemas::BaseSchema;
use crate::utils::format_marker;
use crate::validation::Context;
use crate::validation::Validator;

/// A number schema
#[derive(Debug, PartialEq)]
pub struct NumberSchema {
    pub base: BaseSchema,
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

impl Default for NumberSchema {
    fn default() -> Self {
        Self {
            base: BaseSchema::type_number(),
            minimum: None,
            maximum: None,
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
        }
    }
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
                self.validate_number_i64(context, &enum_values, value, *i)
            } else if let Scalar::FloatingPoint(o) = scalar {
                let enum_values = self.base.r#enum.as_ref().map(|r#enum| {
                    r#enum
                        .iter()
                        .filter_map(|v| {
                            if let ConstValue::Number(Number::Float(f)) = v {
                                Some(*f)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<f64>>()
                });
                self.validate_number_f64(context, &enum_values, value, o.into_inner())
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
    pub fn from_base(base: BaseSchema) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    // TODO: This duplicates IntegerSchema::validate_integer(), so, find a neat way to dedupe this
    fn validate_number_i64(
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
        let mut number_schema = NumberSchema::from_base(BaseSchema::try_from(mapping)?);
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
                    // These should've been handled by the base schema
                    "type" => (),
                    "enum" => (),
                    "const" => (),
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
    }
}
