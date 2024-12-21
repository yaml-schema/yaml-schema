use regex::Regex;

use crate::Context;
use crate::Result;
use crate::StringSchema;

use super::Validator;

impl Validator for StringSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let errors = validate_string(
            self.min_length,
            self.max_length,
            self.pattern.as_ref(),
            self.r#enum.as_ref(),
            value,
        );
        if !errors.is_empty() {
            for error in errors {
                context.add_error(value, error);
            }
        }
        Ok(())
    }
}

/// Just trying to isolate the actual validation into a function that doesn't take a context
pub fn validate_string(
    min_length: Option<usize>,
    max_length: Option<usize>,
    pattern: Option<&Regex>,
    r#enum: Option<&Vec<String>>,
    value: &saphyr::MarkedYaml,
) -> Vec<String> {
    let mut errors = Vec::new();
    let data = &value.data;
    let str_value = match data.as_str() {
        Some(s) => s,
        None => {
            errors.push(format!("Expected a string, but got: {:?}", data));
            return errors;
        }
    };
    if let Some(min_length) = min_length {
        if str_value.len() < min_length {
            errors.push(format!("String is too short! (min length: {})", min_length));
        }
    }
    if let Some(max_length) = max_length {
        if str_value.len() > max_length {
            errors.push(format!("String is too long! (max length: {})", max_length));
        }
    }
    if let Some(regex) = pattern {
        if !regex.is_match(str_value) {
            errors.push(format!(
                "String does not match regular expression {}!",
                regex.as_str()
            ));
        }
    }
    if let Some(enum_values) = r#enum {
        if !enum_values.contains(&str_value.to_string()) {
            errors.push(format!("String is not in enum: {:?}", enum_values));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use crate::Engine;
    use crate::RootSchema;
    use crate::Schema;

    use super::*;

    #[test]
    fn test_engine_validate_string() {
        let schema = StringSchema::default();
        let root_schema = RootSchema::new_with_schema(Schema::String(schema));
        let context = Engine::evaluate(&root_schema, "some string", false).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_engine_validate_string_with_min_length() {
        let schema = StringSchema {
            min_length: Some(5),
            ..Default::default()
        };
        let root_schema = RootSchema::new_with_schema(Schema::String(schema));
        let context = Engine::evaluate(&root_schema, "hello", false).unwrap();
        assert!(!context.has_errors());
        let context = Engine::evaluate(&root_schema, "hell", false).unwrap();
        assert!(context.has_errors());
    }

    #[test]
    fn test_validate_string() {
        let docs = saphyr::MarkedYaml::load_from_str("hello").unwrap();
        let value = docs.first().unwrap();
        let errors = validate_string(None, None, None, None, value);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_string_with_min_length() {
        let docs = saphyr::MarkedYaml::load_from_str("hello").unwrap();
        let errors = validate_string(Some(5), None, None, None, docs.first().unwrap());
        assert!(errors.is_empty());
        let docs = saphyr::MarkedYaml::load_from_str("hell").unwrap();
        let errors = validate_string(Some(5), None, None, None, docs.first().unwrap());
        assert!(!errors.is_empty());
        let first = errors.first().unwrap();
        assert_eq!(first, "String is too short! (min length: 5)");
    }

    #[test]
    fn test_string_schema_validation() {
        let schema = StringSchema::default();
        let docs = saphyr::MarkedYaml::load_from_str("Washington").unwrap();
        let value = docs.first().unwrap();
        let context = Context::default();
        let result = schema.validate(&context, value);
        assert!(result.is_ok());
    }
}
