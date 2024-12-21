use log::{debug, error};

use super::Validator;
use crate::Context;
use crate::Error;
use crate::Result;
use crate::YamlSchema;

impl Validator for crate::schemas::AnyOfSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let any_of_is_valid = validate_any_of(&self.any_of, context, value)?;
        if !any_of_is_valid {
            error!("AnyOf: None of the schemas in `oneOf` matched!");
            context.add_error(value, "None of the schemas in `oneOf` matched!");
            fail_fast!(context);
        }
        Ok(())
    }
}

pub fn validate_any_of(
    schemas: &Vec<YamlSchema>,
    context: &Context,
    value: &saphyr::MarkedYaml,
) -> Result<bool> {
    for schema in schemas {
        debug!(
            "AnyOf: Validating value: {:?} against schema: {}",
            value, schema
        );
        // Since we're only looking for the first match, we can stop as soon as we find one
        // That also means that when evaluating sub schemas, we can fail fast to short circuit
        // the rest of the validation
        let sub_context = context.get_sub_context();
        let sub_result = schema.validate(&sub_context, value);
        match sub_result {
            Ok(()) | Err(Error::FailFast) => {
                if sub_context.has_errors() {
                    continue;
                }
                return Ok(true);
            }
            Err(e) => return Err(e),
        }
    }
    // If we get here, then none of the schemas matched
    Ok(false)
}
