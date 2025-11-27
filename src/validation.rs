//! The validation module contains the logic for validating a YAML schema against a YAML value

use saphyr::Marker;

use crate::Result;

mod context;
mod objects;
mod strings;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::YamlSchema;
    use saphyr::LoadableYamlNode;

    #[test]
    fn test_validate_empty_schema() {
        let schema = YamlSchema::Empty;
        let context = Context::default();
        let docs = saphyr::MarkedYaml::load_from_str("value").unwrap();
        let value = docs.first().unwrap();
        let result = schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(!context.has_errors());
    }

    #[test]
    fn test_validate_type_null() {
        let schema = YamlSchema::Null;
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
