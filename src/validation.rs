use log::debug;

use saphyr::Marker;

pub mod any_of;
mod context;
mod objects;
mod one_of;
mod strings;

use crate::Result;
use crate::Schema;
use crate::utils::format_yaml_data;

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

/// Display these ValidationErrors as "{path}: {error}"
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
            Schema::BooleanLiteral(boolean) => {
                if !*boolean {
                    context.add_error(value, "Schema is `false`!".to_string());
                }
                Ok(())
            }
            Schema::Const(const_schema) => const_schema.validate(context, value),
            Schema::Enum(enum_schema) => enum_schema.validate(context, value),
            Schema::AllOf(all_of_schema) => all_of_schema.validate(context, value),
            Schema::AnyOf(any_of_schema) => any_of_schema.validate(context, value),
            Schema::OneOf(one_of_schema) => one_of_schema.validate(context, value),
            Schema::Not(not_schema) => not_schema.validate(context, value),
            Schema::Typed(typed_schema) => typed_schema.validate(context, value),
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
