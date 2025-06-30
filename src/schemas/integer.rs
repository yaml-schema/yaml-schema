use log::debug;

use crate::validation::Context;
use crate::validation::Validator;
use crate::Number;
use crate::Result;

/// A number schema
#[derive(Debug, Default, PartialEq)]
pub struct IntegerSchema {
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

impl std::fmt::Display for IntegerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Number {self:?}")
    }
}

impl Validator for IntegerSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[IntegerSchema] self: {}", self);
        debug!("[IntegerSchema] Validating value: {:?}", value);
        let data = &value.data;
        if data.is_integer() {
            match data.as_i64() {
                Some(i) => self.validate_number_i64(context, value, i),
                None => {
                    context.add_error(value, format!("Expected an integer, but got: {data:?}"));
                }
            }
        } else if data.is_real() {
            match data.as_f64() {
                Some(f) => {
                    if f.fract() == 0.0 {
                        self.validate_number_i64(context, value, f as i64);
                    } else {
                        context.add_error(value, format!("Expected an integer, but got: {data:?}"));
                    }
                }
                None => {
                    context.add_error(value, format!("Expected a float, but got: {data:?}"));
                }
            }
        } else {
            context.add_error(value, format!("Expected a number, but got: {data:?}"));
        }
        if !context.errors.borrow().is_empty() {
            fail_fast!(context)
        }
        Ok(())
    }
}

impl IntegerSchema {
    fn validate_number_i64(&self, context: &Context, value: &saphyr::MarkedYaml, i: i64) {
        if let Some(minimum) = &self.minimum {
            match minimum {
                Number::Integer(min) => {
                    if i < *min {
                        context.add_error(value, "Number is too small!".to_string());
                    }
                }
                Number::Float(min) => {
                    if (i as f64) < *min {
                        context.add_error(value, "Number is too small!".to_string());
                    }
                }
            }
        }
        if let Some(maximum) = &self.maximum {
            match maximum {
                Number::Integer(max) => {
                    if i > *max {
                        context.add_error(value, "Number is too big!".to_string());
                    }
                }
                Number::Float(max) => {
                    if (i as f64) > *max {
                        context.add_error(value, "Number is too big!".to_string());
                    }
                }
            }
        }
        if let Some(multiple_of) = &self.multiple_of {
            match multiple_of {
                Number::Integer(multiple) => {
                    if i % *multiple != 0 {
                        context
                            .add_error(value, format!("Number is not a multiple of {multiple}!"));
                    }
                }
                Number::Float(multiple) => {
                    if (i as f64) % *multiple != 0.0 {
                        context
                            .add_error(value, format!("Number is not a multiple of {multiple}!"));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
            "Expected a number, but got: String(\"foo\")"
        );
    }
}
