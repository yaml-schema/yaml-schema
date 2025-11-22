use log::debug;
use regex::Regex;

use crate::ConstValue;
use crate::Context;
use crate::Result;
use crate::StringSchema;
use crate::Validator;

impl Validator for StringSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let errors = self.do_validate(value);
        if !errors.is_empty() {
            for error in errors {
                context.add_error(value, error);
            }
        }
        Ok(())
    }
}

impl StringSchema {
    fn do_validate(&self, value: &saphyr::MarkedYaml) -> Vec<String> {
        debug!("do_validate: {:?}", value.data);
        let mut errors = Vec::new();

        if let saphyr::YamlData::Value(scalar) = &value.data
            && let saphyr::Scalar::String(s) = scalar
        {
            let enum_strings = self.base.r#enum.as_ref().map(|enum_values| {
                enum_values
                    .iter()
                    .filter_map(|v| {
                        if let ConstValue::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            });
            debug!("enum_strings: {enum_strings:?}");
            validate_string(
                &mut errors,
                self.min_length,
                self.max_length,
                self.pattern.as_ref(),
                enum_strings.as_ref(),
                s,
            );
        } else {
            errors.push(format!("Expected a string, but got: {:?}", value.data));
        }
        errors
    }
}

/// Just trying to isolate the actual validation into a function that doesn't take a context
pub fn validate_string(
    errors: &mut Vec<String>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    pattern: Option<&Regex>,
    r#enum: Option<&Vec<String>>,
    str_value: &str,
) {
    if let Some(min_length) = min_length
        && str_value.len() < min_length
    {
        errors.push(format!("String is too short! (min length: {min_length})"));
    }
    if let Some(max_length) = max_length
        && str_value.len() > max_length
    {
        errors.push(format!("String is too long! (max length: {max_length})"));
    }
    if let Some(regex) = pattern
        && !regex.is_match(str_value)
    {
        errors.push(format!(
            "String does not match regular expression {}!",
            regex.as_str()
        ));
    }
    if let Some(enum_values) = r#enum
        && !enum_values.contains(&str_value.to_string())
    {
        errors.push(format!("String is not in enum: {enum_values:?}"));
    }
}

#[cfg(test)]
mod tests {
    use crate::Engine;
    use crate::RootSchema;
    use crate::Schema;
    use saphyr::LoadableYamlNode;

    use super::*;

    #[test]
    fn test_engine_validate_string() {
        let schema = StringSchema::default();
        let root_schema = RootSchema::new_with_schema(Schema::typed_string(schema));
        let context = Engine::evaluate(&root_schema, "some string", false).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_engine_validate_string_with_min_length() {
        let schema = StringSchema {
            min_length: Some(5),
            ..Default::default()
        };
        let root_schema = RootSchema::new_with_schema(Schema::typed_string(schema));
        let context = Engine::evaluate(&root_schema, "hello", false).unwrap();
        assert!(!context.has_errors());
        let context = Engine::evaluate(&root_schema, "hell", false).unwrap();
        assert!(context.has_errors());
    }

    #[test]
    fn test_validate_string() {
        let mut errors = Vec::new();
        validate_string(&mut errors, None, None, None, None, "hello");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_string_with_min_length() {
        let mut errors = Vec::new();
        validate_string(&mut errors, Some(5), None, None, None, "hello");
        assert!(errors.is_empty());
        validate_string(&mut errors, Some(5), None, None, None, "hell");
        assert!(!errors.is_empty());
        assert_eq!(
            errors.first().unwrap(),
            "String is too short! (min length: 5)"
        );
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

    #[test]
    fn test_string_schema_doesnt_validate_object() {
        let yaml = "an: [arbitrarily, nested, data, structure]";
        let doc = saphyr::MarkedYaml::load_from_str(yaml).unwrap();
        let marked_yaml = doc.first().unwrap();
        let string_schema: StringSchema = StringSchema::default();
        let context = Context::default();
        let result = string_schema.validate(&context, marked_yaml);
        assert!(result.is_ok());
        assert!(context.has_errors());
    }
}
