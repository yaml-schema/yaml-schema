use log::debug;
use std::cmp::Ordering;

use saphyr::Marker;

pub mod any_of;
mod context;
mod objects;
mod one_of;
mod strings;

use crate::utils::format_yaml_data;
use crate::Number;
use crate::Result;
use crate::Schema;

pub use context::Context;

/// A trait for validating a sahpyr::Yaml value against a schema
pub trait Validator {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()>;
}

/// A validation error simply contains a path and an error message
#[derive(Debug)]
pub struct ValidationError {
    /// The path to the value that caused the error
    pub path: String,
    /// The line and column of the value that caused the error
    pub marker: Option<Marker>,
    /// The error message
    pub error: String,
}

/// Display this ValidationErrors as "{path}: {error}"
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(marker) = &self.marker {
            write!(
                f,
                "[{}:{}] .{}: {}",
                marker.line(),
                marker.col() + 1, // contrary to the documentation, columns are 0-indexed
                self.path,
                self.error
            )
        } else {
            write!(f, ".{}: {}", self.path, self.error)
        }
    }
}

impl Validator for Schema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[Schema] self: {self}");
        debug!(
            "[Schema] Validating value: {}",
            format_yaml_data(&value.data)
        );
        match self {
            Schema::Empty => Ok(()),
            Schema::TypeNull => {
                if !value.data.is_null() {
                    context.add_error(value, format!("Expected null, but got: {:?}", value.data));
                }
                Ok(())
            }
            Schema::BooleanLiteral(boolean) => {
                if !*boolean {
                    context.add_error(value, "Schema is `false`!".to_string());
                }
                Ok(())
            }
            Schema::BooleanSchema => validate_boolean_schema(context, value),
            Schema::Const(const_schema) => const_schema.validate(context, value),
            Schema::Enum(enum_schema) => enum_schema.validate(context, value),
            Schema::Integer(integer_schema) => integer_schema.validate(context, value),
            Schema::String(string_schema) => string_schema.validate(context, value),
            Schema::Number(number_schema) => number_schema.validate(context, value),
            Schema::Object(object_schema) => object_schema.validate(context, value),
            Schema::Array(array_schema) => array_schema.validate(context, value),
            Schema::AnyOf(any_of_schema) => any_of_schema.validate(context, value),
            Schema::OneOf(one_of_schema) => one_of_schema.validate(context, value),
            Schema::Not(not_schema) => not_schema.validate(context, value),
        }
    }
}

fn validate_boolean_schema(context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
    if !value.data.is_boolean() {
        context.add_error(value, format!("Expected: boolean, found: {value:?}"));
    }
    Ok(())
}

pub fn validate_integer(
    context: &Context,
    minimum: &Option<Number>,
    maximum: &Option<Number>,
    exclusive_minimum: &Option<Number>,
    exclusive_maximum: &Option<Number>,
    multiple_of: &Option<Number>,
    value: &saphyr::MarkedYaml,
    i: i64,
) {
    if let Some(exclusive_min) = exclusive_minimum {
        match exclusive_min {
            Number::Integer(exclusive_min) => {
                if i <= *exclusive_min {
                    context.add_error(
                        value,
                        format!("Number must be greater than {exclusive_min}"),
                    );
                }
            }
            Number::Float(exclusive_min) => {
                if (i as f64).partial_cmp(exclusive_min) != Some(Ordering::Greater) {
                    context.add_error(
                        value,
                        format!("Number must be greater than {exclusive_min}"),
                    );
                }
            }
        }
    } else if let Some(minimum) = minimum {
        match minimum {
            Number::Integer(min) => {
                if i <= *min {
                    context.add_error(
                        value,
                        format!("Number must be greater than or equal to {min}"),
                    );
                }
            }
            Number::Float(min) => {
                let cmp = (i as f64).partial_cmp(min);
                if cmp != Some(Ordering::Less) && cmp != Some(Ordering::Equal) {
                    context.add_error(
                        value,
                        format!("Number must be greater than or equal to {min}"),
                    );
                }
            }
        }
    }

    if let Some(exclusive_max) = exclusive_maximum {
        match exclusive_max {
            Number::Integer(exclusive_max) => {
                if i >= *exclusive_max {
                    context.add_error(
                        value,
                        format!("Number must be less than than {exclusive_max}"),
                    );
                }
            }
            Number::Float(exclusive_max) => {
                if (i as f64).partial_cmp(exclusive_max) != Some(Ordering::Less) {
                    context.add_error(
                        value,
                        format!("Number must be less than than {exclusive_max}"),
                    );
                }
            }
        }
    } else if let Some(maximum) = maximum {
        match maximum {
            Number::Integer(max) => {
                if i >= *max {
                    context.add_error(value, format!("Number must be less than or equal to {max}"));
                }
            }
            Number::Float(max) => {
                let cmp = (i as f64).partial_cmp(max);
                if cmp != Some(Ordering::Greater) && cmp != Some(Ordering::Equal) {
                    context.add_error(value, format!("Number must be less than or equal to {max}"));
                }
            }
        }
    }

    if let Some(multiple_of) = &multiple_of {
        match multiple_of {
            Number::Integer(multiple) => {
                if i % *multiple != 0 {
                    context.add_error(value, format!("Number is not a multiple of {multiple}!"));
                }
            }
            Number::Float(multiple) => {
                if (i as f64) % *multiple != 0.0 {
                    context.add_error(value, format!("Number is not a multiple of {multiple}!"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::YamlSchema;
    use saphyr::LoadableYamlNode;

    #[test]
    fn test_validate_empty_schema() {
        let schema = YamlSchema::empty();
        let context = Context::default();
        let docs = saphyr::MarkedYaml::load_from_str("value").unwrap();
        let value = docs.first().unwrap();
        let result = schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(!context.has_errors());
    }

    #[test]
    fn test_validate_type_null() {
        let schema = YamlSchema::null();
        let context = Context::default();
        let docs = saphyr::MarkedYaml::load_from_str("value").unwrap();
        let value = docs.first().unwrap();
        let result = schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        let error = errors.first().unwrap();
        assert_eq!(
            error.error,
            "Expected null, but got: Value(String(\"value\"))"
        );
    }
}
