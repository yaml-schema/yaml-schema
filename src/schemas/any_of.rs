use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::YamlData;

use crate::Context;
use crate::Error;
use crate::Result;
use crate::Validator;
use crate::YamlSchema;
use crate::loader;
use crate::utils::format_vec;

/// The `anyOf` schema is a schema that matches if any of the schemas in the `anyOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
#[derive(Debug, Default, PartialEq)]
pub struct AnyOfSchema {
    pub any_of: Vec<YamlSchema>,
}

impl std::fmt::Display for AnyOfSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "anyOf:{}", format_vec(&self.any_of))
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for AnyOfSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml<'r>) -> Result<Self> {
        if let YamlData::Mapping(mapping) = &value.data {
            AnyOfSchema::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for AnyOfSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'r, MarkedYaml<'r>>) -> crate::Result<Self> {
        let mut any_of_schema = AnyOfSchema::default();
        if let Some(value) = mapping.get(&MarkedYaml::value_from_str("anyOf")) {
            any_of_schema.any_of = loader::load_array_of_schemas_marked(value)?;
        } else {
            debug!("[anyOf] No `anyOf` key found!");
        }
        Ok(any_of_schema)
    }
}

impl Validator for crate::schemas::AnyOfSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let any_of_is_valid = validate_any_of(&self.any_of, context, value)?;
        debug!("any_of_is_valid: {any_of_is_valid}");
        if !any_of_is_valid {
            debug!("AnyOf: None of the schemas in `anyOf` matched!");
            context.add_error(value, "None of the schemas in `anyOf` matched!");
            fail_fast!(context);
        }
        Ok(())
    }
}

pub fn validate_any_of(
    schemas: &[YamlSchema],
    context: &Context,
    marked_yaml: &saphyr::MarkedYaml,
) -> Result<bool> {
    debug!("[AnyOf] &context: {context:p}");
    let mut any_ok = false;
    for schema in schemas {
        debug!("[AnyOf] Validating value: {marked_yaml:?} against schema: {schema}");
        let sub_context = context.get_sub_context_fresh_eval();
        debug!("[AnyOf]     context: {context:?}");
        debug!("[AnyOf] sub_context: {sub_context:?}");
        match schema.validate(&sub_context, marked_yaml) {
            Ok(()) | Err(Error::FailFast) => {
                if sub_context.has_errors() {
                    continue;
                }
                debug!("[AnyOf] Schema {schema:?} matched");
                any_ok = true;
                if let (Some(p), Some(b)) =
                    (&context.object_evaluated, &sub_context.object_evaluated)
                {
                    p.extend(&b.snapshot());
                }
                if let (Some(pcell), Some(bcell)) =
                    (&context.array_unevaluated, &sub_context.array_unevaluated)
                {
                    let snap = bcell.borrow().clone();
                    pcell.borrow_mut().merge_from(&snap);
                }
            }
            Err(e) => return Err(e),
        }
    }
    debug!("[AnyOf] any_ok: {any_ok}");
    Ok(any_ok)
}

#[cfg(test)]
mod tests {
    use saphyr::MarkedYaml;

    use crate::Context;
    use crate::Validator as _;
    use crate::loader;

    #[test]
    fn test_any_of_with_description() {
        let schema_str = r#"
        description: A string or a number
        anyOf:
          - type: string
          - type: number
        "#;
        let any_of_schema = loader::load_from_str(schema_str).expect("Failed to load schema");

        // Test string
        let value_str = r#""I am a string""#;
        let value = MarkedYaml::value_from_str(value_str);
        assert!(value.data.is_string(), "Value should be a string");
        let context = Context::default();
        any_of_schema
            .validate(&context, &value)
            .expect("Validation failed");
        assert!(!context.has_errors(), "Should accept string");

        // Test number
        let value_str = "42";
        let value = MarkedYaml::value_from_str(value_str);
        assert!(value.data.is_integer(), "Value should be an integer");
        let context = Context::default();
        any_of_schema
            .validate(&context, &value)
            .expect("Validation failed");
        assert!(!context.has_errors(), "Should accept number");

        // Test boolean (should fail)
        let value_str = "true";
        let value = MarkedYaml::value_from_str(value_str);
        assert!(value.data.is_boolean(), "Value should be a boolean");
        let context = Context::default();
        any_of_schema
            .validate(&context, &value)
            .expect("Validation failed");
        assert!(context.has_errors(), "Should NOT accept boolean");
    }
}
