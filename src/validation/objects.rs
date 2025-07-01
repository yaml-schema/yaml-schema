use crate::schemas::BoolOrTypedSchema;
use crate::schemas::ObjectSchema;
use crate::utils::saphyr_yaml_string;
use crate::validation::Context;
use crate::Error;
use crate::Result;
use crate::Validator;
use crate::YamlSchema;
use crate::{format_marker, format_yaml_data};
/// A module to contain object type validation logic
use log::debug;
use saphyr::MarkedYaml;
use std::collections::HashMap;

impl Validator for ObjectSchema {
    /// Validate the object according to the schema rules
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let data = &value.data;
        debug!("Validating object: {}", format_yaml_data(data));
        if let saphyr::YamlData::Mapping(mapping) = data {
            self.validate_object_mapping(context, value, mapping)
        } else {
            context.add_error(value, format!("Expected an object, but got: {data:#?}"));
            Ok(())
        }
    }
}

pub fn try_validate_value_against_properties(
    context: &Context,
    key: &String,
    value: &saphyr::MarkedYaml,
    properties: &HashMap<String, YamlSchema>,
) -> Result<bool> {
    let sub_context = context.append_path(key);
    if let Some(schema) = properties.get(key) {
        debug!("Validating property '{}' with schema: {}", key, schema);
        let result = schema.validate(&sub_context, value);
        return match result {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        };
    }
    Ok(false)
}

/// Try and validate the value against an object type's additional_properties
///
/// Returns true if the validation passed, or false if it failed (signals fail-fast)
pub fn try_validate_value_against_additional_properties(
    context: &Context,
    key: &String,
    value: &saphyr::MarkedYaml,
    additional_properties: &BoolOrTypedSchema,
) -> Result<bool> {
    let sub_context = context.append_path(key);

    match additional_properties {
        // if additional_properties: true, then any additional properties are allowed
        BoolOrTypedSchema::Boolean(true) => { /* noop */ }
        // if additional_properties: false, then no additional properties are allowed
        BoolOrTypedSchema::Boolean(false) => {
            context.add_error(
                value,
                format!("Additional property '{key}' is not allowed!"),
            );
            // returning `false` signals fail fast
            return Ok(false);
        }
        // if additional_properties: a schema, then validate against it
        BoolOrTypedSchema::TypedSchema(schema) => {
            schema.validate(&sub_context, value)?;
        }
        BoolOrTypedSchema::Reference(reference) => {
            // Grab the reference from the root schema.
            let Some(root) = &context.root_schema else {
                context.add_error(
                    value,
                    "No root schema was provided to look up references".to_string(),
                );
                return Ok(false);
            };
            let Some(def) = root.get_def(&reference.ref_name) else {
                context.add_error(
                    value,
                    format!("No definition for {} found", reference.ref_name),
                );
                return Ok(false);
            };

            def.validate(context, value)?;
        }
    }
    Ok(true)
}

impl ObjectSchema {
    fn validate_object_mapping<'a>(
        &self,
        context: &Context,
        object: &saphyr::MarkedYaml,
        mapping: &saphyr::AnnotatedMapping<'a, MarkedYaml<'a>>,
    ) -> Result<()> {
        for (k, value) in mapping {
            let key = match &k.data {
                saphyr::YamlData::Value(scalar) => match scalar {
                    saphyr::Scalar::String(s) => s.to_string(),
                    v => return Err(generic_error!("Expected a string key, got: {:?}", v)),
                },
                v => return Err(expected_scalar!("Expected a string key, got: {:?}", v)),
            };
            let span = &k.span;
            debug!("validate_object_mapping: key: \"{}\"", key);
            debug!(
                "validate_object_mapping: span.start: {:?}",
                format_marker(&span.start)
            );
            debug!(
                "validate_object_mapping: span.end: {:?}",
                format_marker(&span.end)
            );
            // First, we check the explicitly defined properties, and validate against it if found
            if let Some(properties) = &self.properties {
                if try_validate_value_against_properties(context, &key, value, properties)? {
                    continue;
                }
            }

            // Then, we check if additional properties are allowed or not
            if let Some(additional_properties) = &self.additional_properties {
                try_validate_value_against_additional_properties(
                    context,
                    &key,
                    value,
                    additional_properties,
                )?;
            }

            // Then we check if pattern_properties matches
            if let Some(pattern_properties) = &self.pattern_properties {
                for (pattern, schema) in pattern_properties {
                    log::debug!("pattern: {}", pattern);
                    // TODO: compile the regex once instead of every time we're evaluating
                    let re = regex::Regex::new(pattern).map_err(|e| {
                        Error::GenericError(format!("Invalid regular expression pattern: {e}"))
                    })?;
                    if re.is_match(key.as_str()) {
                        schema.validate(context, value)?;
                    }
                }
            }
            // Finally, we check if it matches property_names
            if let Some(property_names) = &self.property_names {
                let re = regex::Regex::new(property_names).map_err(|e| {
                    Error::GenericError(format!("Invalid regular expression pattern: {e}"))
                })?;
                debug!("Regex for property names: {}", re.as_str());
                if !re.is_match(key.as_str()) {
                    context.add_error(
                        k,
                        format!(
                            "Property name '{}' does not match pattern '{}'",
                            key,
                            re.as_str()
                        ),
                    );
                    fail_fast!(context)
                }
            }
        }
        // If we have any AnyOf specification, check the object format against one of them.
        if let Some(any_of) = &self.any_of {
            any_of.validate(context, object)?;
        }

        // Validate required properties
        if let Some(required) = &self.required {
            for required_property in required {
                if !mapping
                    .keys()
                    .map(|k| k.data.as_str().unwrap())
                    .any(|s| s == required_property)
                {
                    context.add_error(
                        object,
                        format!("Required property '{required_property}' is missing!"),
                    );
                    fail_fast!(context)
                }
            }
        }

        // Validate minProperties
        if let Some(min_properties) = &self.min_properties {
            if mapping.len() < *min_properties {
                context.add_error(
                    object,
                    format!("Object has too few properties! Minimum is {min_properties}!"),
                );
                fail_fast!(context)
            }
        }
        // Validate maxProperties
        if let Some(max_properties) = &self.max_properties {
            if mapping.len() > *max_properties {
                context.add_error(
                    object,
                    format!("Object has too many properties! Maximum is {max_properties}!"),
                );
                fail_fast!(context)
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::engine;
    use crate::NumberSchema;
    use crate::RootSchema;
    use crate::Schema;
    use crate::StringSchema;

    use super::*;

    #[test]
    fn test_should_validate_properties() {
        let mut properties = HashMap::new();
        properties.insert(
            "foo".to_string(),
            YamlSchema::from(Schema::String(StringSchema::default())),
        );
        properties.insert(
            "bar".to_string(),
            YamlSchema::from(Schema::Number(NumberSchema::default())),
        );
        let object_schema = ObjectSchema {
            properties: Some(properties),
            ..Default::default()
        };
        let root_schema = RootSchema::new_with_schema(Schema::Object(object_schema));
        let value = r#"
            foo: "I'm a string"
            bar: 42
        "#;
        let result = engine::Engine::evaluate(&root_schema, value, true);
        assert!(result.is_ok());

        let value2 = r#"
            foo: 42
            baz: "I'm a string"
        "#;
        let context = engine::Engine::evaluate(&root_schema, value2, true).unwrap();
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        let first_error = errors.first().unwrap();
        assert_eq!(first_error.path, "foo");
        assert_eq!(first_error.error, "Expected a string, but got: Integer(42)");
    }
}
