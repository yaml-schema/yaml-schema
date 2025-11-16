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
            error!("AnyOf: None of the schemas in `anyOf` matched!");
            context.add_error(value, "None of the schemas in `anyOf` matched!");
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
        debug!("AnyOf: Validating value: {value:?} against schema: {schema}");
        // Since we're only looking for the first match, we can stop as soon as we find one
        // That also means that when evaluating sub schemas, we can fail fast to short circuit
        // the rest of the validation
        let sub_context = context.get_sub_context();
        match schema.validate(&sub_context, value) {
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
