use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::YamlData;

use crate::Context;
use crate::Result;
use crate::Validator;
use crate::YamlSchema;

/// The `not` keyword declares that an instance validates if it doesn't validate against the given subschema.
#[derive(Debug, PartialEq)]
pub struct NotSchema {
    pub not: Box<YamlSchema>,
}

impl std::fmt::Display for NotSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not: {}", self.not)
    }
}

impl TryFrom<&MarkedYaml<'_>> for NotSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml<'_>) -> Result<Self> {
        if let YamlData::Mapping(mapping) = &value.data {
            NotSchema::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for NotSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        if let Some(value) = mapping.get(&MarkedYaml::value_from_str("not")) {
            let schema: YamlSchema = value.try_into()?;
            Ok(NotSchema {
                not: Box::new(schema),
            })
        } else {
            Err(generic_error!("No `not` key found!"))
        }
    }
}

impl Validator for NotSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> crate::Result<()> {
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
