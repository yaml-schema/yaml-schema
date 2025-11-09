use crate::loader::{FromAnnotatedMapping, FromSaphyrMapping};
use crate::{Context, Validator, YamlSchema, loader};
use log::debug;
use saphyr::{AnnotatedMapping, MarkedYaml, Scalar, YamlData};

/// The `not` keyword declares that an instance validates if it doesn't validate against the given subschema.
#[derive(Debug, Default, PartialEq)]
pub struct NotSchema {
    pub not: Box<YamlSchema>,
}

impl std::fmt::Display for NotSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not: {}", self.not)
    }
}

impl FromSaphyrMapping<NotSchema> for NotSchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> crate::Result<NotSchema> {
        let mut not_schema = NotSchema::default();
        for (key, value) in mapping.iter() {
            if let Ok(key) = loader::load_string_value(key) {
                match key.as_str() {
                    "not" => {
                        if let saphyr::Yaml::Mapping(mapping) = value {
                            let schema = YamlSchema::from_mapping(mapping)?;
                            not_schema.not = Box::new(schema);
                        } else {
                            return Err(generic_error!("Expected a mapping, but got: {:#?}", key));
                        }
                    }
                    _ => return Err(generic_error!("Unsupported key: {}", key)),
                }
            }
        }
        Ok(not_schema)
    }
}

impl FromAnnotatedMapping<NotSchema> for NotSchema {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> crate::Result<NotSchema> {
        let mut not_schema = NotSchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(key)) = &key.data {
                match key.as_ref() {
                    "not" => {
                        let schema: YamlSchema = value.try_into()?;
                        not_schema.not = Box::new(schema);
                    }
                    _ => return Err(generic_error!("Unsupported key: {}", key)),
                }
            }
        }
        Ok(not_schema)
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
