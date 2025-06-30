use crate::Number;
use log::debug;

use crate::ConstValue;
use crate::Context;
use crate::Result;

use super::Validator;

/// A const schema represents a constant value
#[derive(Debug, PartialEq)]
pub struct ConstSchema {
    pub r#const: ConstValue,
}

impl std::fmt::Display for ConstSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Const {:?}", self.r#const)
    }
}

impl Validator for ConstSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let data = &value.data;
        debug!(
            "Validating value: {:?} against const: {:?}",
            &data, self.r#const
        );
        let expected_value = &self.r#const;
        match expected_value {
            ConstValue::Boolean(b) => {
                if data.as_bool() != Some(*b) {
                    let error = format!("Const validation failed, expected: {b:?}, got: {data:?}");
                    context.add_error(value, error);
                }
            }
            ConstValue::Null => {
                if !data.is_null() {
                    let error = format!("Const validation failed, expected: null, got: {data:?}");
                    context.add_error(value, error);
                }
            }
            ConstValue::Number(n) => match n {
                Number::Integer(i) => {
                    if data.is_integer() {
                        if data.as_i64() != Some(*i) {
                            let error =
                                format!("Const validation failed, expected: {i:?}, got: {data:?}");
                            context.add_error(value, error);
                        }
                    } else {
                        let error =
                            format!("Const validation failed, expected: {i:?}, got: {data:?}");
                        context.add_error(value, error);
                    }
                }
                Number::Float(f) => {
                    if data.is_real() {
                        if data.as_f64() != Some(*f) {
                            let error =
                                format!("Const validation failed, expected: {f:?}, got: {data:?}");
                            context.add_error(value, error);
                        }
                    } else {
                        let error =
                            format!("Const validation failed, expected: {f:?}, got: {data:?}");
                        context.add_error(value, error);
                    }
                }
            },
            ConstValue::String(s) => {
                if data.as_str() != Some(s) {
                    let error = format!("Const validation failed, expected: {s:?}, got: {data:?}");
                    context.add_error(value, error);
                }
            }
        }
        Ok(())
    }
}
