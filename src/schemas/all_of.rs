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

/// The `allOf` schema is a schema that matches if all of the schemas in the `allOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
#[derive(Debug, Default, PartialEq)]
pub struct AllOfSchema {
    pub all_of: Vec<YamlSchema>,
}

impl std::fmt::Display for AllOfSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "allOf:{}", format_vec(&self.all_of))
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for AllOfSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml<'r>) -> Result<Self> {
        if let YamlData::Mapping(mapping) = &value.data {
            AllOfSchema::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for AllOfSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'r, MarkedYaml<'r>>) -> crate::Result<Self> {
        let all_of = match mapping.get(&MarkedYaml::value_from_str("allOf")) {
            Some(value) => loader::load_array_of_schemas_marked(value)?,
            None => {
                debug!("[allOf] No `allOf` key found!");
                Vec::new()
            }
        };
        Ok(AllOfSchema { all_of })
    }
}

impl Validator for AllOfSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let all_of_is_valid = validate_all_of(&self.all_of, context, value)?;
        debug!("[AllOf#validate] all_of_is_valid: {all_of_is_valid}");
        if !all_of_is_valid {
            debug!("[AllOf#validate] Not all of the schemas in `allOf` matched!");
            context.add_error(value, "Not all of the schemas in `allOf` matched!");
            fail_fast!(context);
        }
        Ok(())
    }
}

pub fn validate_all_of(
    schemas: &[YamlSchema],
    context: &Context,
    value: &saphyr::MarkedYaml,
) -> Result<bool> {
    for schema in schemas {
        debug!("[AllOf#validate_all_of] Validating value: {value:?} against schema: {schema:?}");
        // We can short circuit as soon as any sub schema fails to validate
        let sub_context = context.get_sub_context();
        let sub_result = schema.validate(&sub_context, value);
        match sub_result {
            Ok(()) => {
                debug!("[AllOf#validate_all_of] schema {schema:?} validated");
                debug!(
                    "[AllOf#validate_all_of] sub_context.has_errors(): {}",
                    sub_context.has_errors()
                );
                if sub_context.has_errors() {
                    return Ok(false);
                }
            }
            Err(Error::FailFast) => return Ok(false),
            Err(e) => return Err(e),
        }
    }
    // If we get here, then all of the schemas matched
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::StringSchema;
    use saphyr::LoadableYamlNode;

    fn create_test_schema() -> AllOfSchema {
        AllOfSchema {
            all_of: vec![
                StringSchema::builder().min_length(1).build().into(),
                StringSchema::builder().max_length(5).build().into(),
            ],
        }
    }

    #[test]
    fn test_validate_all_of() {
        let schema = create_test_schema();
        let context = Context::default();
        let docs = MarkedYaml::load_from_str("valid").unwrap();
        let value = docs.first().unwrap();

        let result = schema.validate(&context, value);

        assert!(result.is_ok());
        assert!(!context.has_errors());
    }

    #[test]
    fn test_validate_all_of_invalid() {
        let schema = create_test_schema();
        let context = Context::default();
        let docs = MarkedYaml::load_from_str("too long").unwrap();
        let value = docs.first().unwrap();

        let result = schema.validate(&context, value);

        assert!(result.is_ok());
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        let error = errors.first().unwrap();
        assert_eq!("Not all of the schemas in `allOf` matched!", error.error);
    }
}
