use crate::schemas::NotSchema;
use crate::validation::Context;
use crate::validation::Validator;
use crate::Result;
use log::debug;

impl Validator for NotSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!(
            "Not: Validating value: {:?} against schema: {}",
            value, self.not
        );

        // Create a sub-context to validate against the inner schema
        let sub_context = context.get_sub_context();
        let sub_result = self.not.validate(&sub_context, value);

        match sub_result {
            Ok(()) | Err(crate::Error::FailFast) => {
                // If the inner schema validates successfully, then this is an error for 'not'
                if !sub_context.has_errors() {
                    context.add_error(value, "Value matches schema in `not`");
                    fail_fast!(context);
                }
            }
            Err(e) => return Err(e),
        }

        // If we get here, then the inner schema failed validation, which means
        // this 'not' validation succeeds
        Ok(())
    }
}
