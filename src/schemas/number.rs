use crate::validation::Context;
use crate::validation::Validator;
use crate::Number;
use crate::Result;

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
        if let saphyr::YamlData::Value(scalar) = data {
            if let saphyr::Scalar::Integer(i) = scalar {
                self.validate_number_i64(context, value, *i)
            } else if let saphyr::Scalar::FloatingPoint(o) = scalar {
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
    fn validate_number_i64(&self, context: &Context, value: &saphyr::MarkedYaml, i: i64) {
        crate::validation::validate_integer(
            context,
            &self.minimum,
            &self.maximum,
            &self.multiple_of,
            value,
            i,
        )
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
