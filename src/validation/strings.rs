use log::debug;
use regex::Regex;

use crate::Context;
use crate::Result;
use crate::Validator;
use crate::schemas::StringFormat;
use crate::schemas::StringSchema;
use crate::validation::formats;

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
            // TODO: add enum validation
            let enum_strings = None;
            debug!("enum_strings: {enum_strings:?}");
            validate_string(
                &mut errors,
                self.min_length,
                self.max_length,
                self.pattern.as_ref(),
                self.format.as_ref(),
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
    format: Option<&StringFormat>,
    r#enum: Option<&Vec<String>>,
    str_value: &str,
) {
    // JSON Schema string length is the number of Unicode scalar values (JSON / RFC 8259
    // "characters"), not UTF-8 byte length.
    let char_len =
        (min_length.is_some() || max_length.is_some()).then(|| str_value.chars().count());
    if let Some(n) = char_len
        && let Some(min_length) = min_length
        && n < min_length
    {
        errors.push(format!("String is too short! (min length: {min_length})"));
    }
    if let Some(n) = char_len
        && let Some(max_length) = max_length
        && n > max_length
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
    if let Some(fmt) = format
        && let Some(err) = formats::validate_format(fmt, str_value)
    {
        errors.push(err);
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
    use crate::YamlSchema;
    use saphyr::LoadableYamlNode;

    use super::*;

    #[test]
    fn test_engine_validate_string() {
        let schema = StringSchema::default();
        let root_schema = RootSchema::new(YamlSchema::typed_string(schema));
        let context = Engine::evaluate(&root_schema, "some string", false).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_engine_validate_string_with_min_length() {
        let schema = StringSchema {
            min_length: Some(5),
            ..Default::default()
        };
        let root_schema = RootSchema::new(YamlSchema::typed_string(schema));
        let context = Engine::evaluate(&root_schema, "hello", false).unwrap();
        assert!(!context.has_errors());
        let context = Engine::evaluate(&root_schema, "hell", false).unwrap();
        assert!(context.has_errors());
    }

    #[test]
    fn test_validate_string() {
        let mut errors = Vec::new();
        validate_string(&mut errors, None, None, None, None, None, "hello");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_string_with_min_length() {
        let mut errors = Vec::new();
        validate_string(&mut errors, Some(5), None, None, None, None, "hello");
        assert!(errors.is_empty());
        validate_string(&mut errors, Some(5), None, None, None, None, "hell");
        assert!(!errors.is_empty());
        assert_eq!(
            errors.first().unwrap(),
            "String is too short! (min length: 5)"
        );
    }

    /// `minLength` / `maxLength` count Unicode scalars, not UTF-8 bytes (JSON Schema).
    #[test]
    fn test_validate_string_length_counts_unicode_scalars_not_utf8_bytes() {
        // Three Greek letters: 3 characters, 6 UTF-8 bytes.
        let greek = "αβγ";
        assert_eq!(greek.len(), 6);
        assert_eq!(greek.chars().count(), 3);

        let mut errors = Vec::new();
        validate_string(&mut errors, None, Some(3), None, None, None, greek);
        assert!(
            errors.is_empty(),
            "maxLength 3 must allow three characters (not three bytes)"
        );

        let mut errors = Vec::new();
        validate_string(&mut errors, None, Some(2), None, None, None, greek);
        assert_eq!(errors.len(), 1);

        let mut errors = Vec::new();
        validate_string(&mut errors, Some(4), None, None, None, None, greek);
        assert_eq!(
            errors.first().map(|s| s.as_str()),
            Some("String is too short! (min length: 4)")
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

    #[test]
    fn test_validate_string_with_format() {
        let mut errors = Vec::new();
        let fmt = StringFormat::Email;
        validate_string(
            &mut errors,
            None,
            None,
            None,
            Some(&fmt),
            None,
            "user@example.com",
        );
        assert!(errors.is_empty());

        validate_string(
            &mut errors,
            None,
            None,
            None,
            Some(&fmt),
            None,
            "not-an-email",
        );
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("email"));
    }

    #[test]
    fn test_engine_validate_string_with_format() {
        let schema = StringSchema {
            format: Some(StringFormat::Date),
            ..Default::default()
        };
        let root_schema = RootSchema::new(YamlSchema::typed_string(schema));
        let context = Engine::evaluate(&root_schema, "2024-01-15", false).unwrap();
        assert!(!context.has_errors());

        let context = Engine::evaluate(&root_schema, "not-a-date", false).unwrap();
        assert!(context.has_errors());
    }

    #[test]
    fn test_validate_string_unknown_format_always_passes() {
        let mut errors = Vec::new();
        let fmt = StringFormat::Unknown("custom".to_string());
        validate_string(&mut errors, None, None, None, Some(&fmt), None, "anything");
        assert!(errors.is_empty());
    }
}
