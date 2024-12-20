pub mod any_of;
/// Validation engine for YamlSchema
mod context;
mod not;
mod objects;
mod one_of;
mod strings;

use crate::Result;
use crate::Schema;
use crate::YamlSchema;
pub use context::Context;
use log::debug;

/// A trait for validating a sahpyr::Yaml value against a schema
pub trait Validator {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()>;
}

#[derive(Debug)]
pub struct LineCol {
    pub line: usize,
    pub col: usize,
}

impl From<&saphyr::MarkedYaml> for LineCol {
    fn from(value: &saphyr::MarkedYaml) -> Self {
        LineCol {
            line: value.span.start.line(),
            col: value.span.start.col() + 1, // contrary to the documentation, columns are 0-indexed
        }
    }
}

/// A validation error simply contains a path and an error message
#[derive(Debug)]
pub struct ValidationError {
    /// The path to the value that caused the error
    pub path: String,
    /// The line and column of the value that caused the error
    pub line_col: Option<LineCol>,
    /// The error message
    pub error: String,
}

/// Display this ValidationErrors as "{path}: {error}"
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line_col) = &self.line_col {
            write!(
                f,
                "[{}:{}] .{}: {}",
                line_col.line, line_col.col, self.path, self.error
            )
        } else {
            write!(f, ".{}: {}", self.path, self.error)
        }
    }
}

impl Validator for Schema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[Schema] self: {}", self);
        debug!("[Schema] Validating value: {:?}", value);
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

impl Validator for YamlSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[YamlSchema] self: {}", self);
        debug!("[YamlSchema] Validating value: {:?}", value);
        if let Some(reference) = &self.r#ref {
            unimplemented!("Validation of references is not supported yet")
        } else if let Some(schema) = &self.schema {
            schema.validate(context, value)?;
        }
        Ok(())
    }
}

fn validate_boolean_schema(context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
    if !value.data.is_boolean() {
        context.add_error(value, format!("Expected: boolean, found: {:?}", value));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(error.error, "Expected null, but got: String(\"value\")");
    }
}
