use log::debug;
use log::error;
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
use crate::utils::format_yaml_data;

/// The `oneOf` schema is a schema that matches if one, and only one of the schemas in the `oneOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
#[derive(Debug, Default, PartialEq)]
pub struct OneOfSchema<'r> {
    pub one_of: Vec<YamlSchema<'r>>,
}

impl std::fmt::Display for OneOfSchema<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "oneOf:{}", format_vec(&self.one_of))
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for OneOfSchema<'r> {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml<'r>) -> Result<Self> {
        if let YamlData::Mapping(mapping) = &value.data {
            OneOfSchema::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for OneOfSchema<'r> {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'r, MarkedYaml<'r>>) -> Result<Self> {
        debug!("[OneOfSchema#try_from] mapping: {mapping:?}");
        match mapping.get(&MarkedYaml::value_from_str("oneOf")) {
            Some(marked_yaml) => {
                debug!(
                    "[OneOfSchema#try_from] marked_yaml: {}",
                    format_yaml_data(&marked_yaml.data)
                );
                let one_of = loader::load_array_of_schemas_marked(marked_yaml)?;
                Ok(OneOfSchema { one_of })
            }
            None => Err(generic_error!("No `oneOf` key found!")),
        }
    }
}

impl Validator for crate::schemas::OneOfSchema<'_> {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let one_of_is_valid = validate_one_of(context, &self.one_of, value)?;
        if !one_of_is_valid {
            context.add_error(value, "None of the schemas in `oneOf` matched!");
            fail_fast!(context);
        }
        Ok(())
    }
}

pub fn validate_one_of(
    context: &Context,
    schemas: &Vec<YamlSchema<'_>>,
    value: &saphyr::MarkedYaml,
) -> Result<bool> {
    let mut one_of_is_valid = false;
    for schema in schemas {
        debug!(
            "[OneOf] Validating value: {:?} against schema: {}",
            &value.data, schema
        );
        let sub_context = context.get_sub_context();
        let sub_result = schema.validate(&sub_context, value);
        match sub_result {
            Ok(()) | Err(Error::FailFast) => {
                debug!(
                    "[OneOf] sub_context.errors: {}",
                    sub_context.errors.borrow().len()
                );
                if sub_context.has_errors() {
                    continue;
                }

                if one_of_is_valid {
                    error!("[OneOf] Value matched multiple schemas in `oneOf`!");
                    context.add_error(value, "Value matched multiple schemas in `oneOf`!");
                    fail_fast!(context);
                } else {
                    one_of_is_valid = true;
                }
            }
            Err(e) => return Err(e),
        }
    }
    debug!("OneOf: one_of_is_valid: {one_of_is_valid}");
    Ok(one_of_is_valid)
}

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode;
    use saphyr::MarkedYaml;

    use crate::YamlSchema;
    use crate::loader;
    use crate::schemas::SchemaType;

    use super::*;

    #[test]
    fn test_one_of_schema() {
        let yaml = r#"
        oneOf:
          - type: boolean
          - type: integer
        "#;
        let root_schema = loader::load_from_str(yaml).expect("Failed to load schema");
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        let Some(one_of_schema) = &subschema.one_of else {
            panic!("Expected Subschema with oneOf, but got: {subschema:?}");
        };

        if let YamlSchema::Subschema(subschema) = &one_of_schema.one_of[0]
            && let SchemaType::Single(type_value) = &subschema.r#type
        {
            assert_eq!(type_value, "boolean");
        } else {
            panic!(
                "Expected Subschema with type: boolean, but got: {:?}",
                &one_of_schema.one_of[0]
            );
        }

        if let YamlSchema::Subschema(subschema) = &one_of_schema.one_of[1]
            && let SchemaType::Single(type_value) = &subschema.r#type
        {
            assert_eq!(type_value, "integer");
        } else {
            panic!(
                "Expected Subschema with type: integer, but got: {:?}",
                &one_of_schema.one_of[1]
            );
        }

        let s = r#"
            false
            "#;
        let docs = MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::with_root_schema(&root_schema, false);
        let result = root_schema.validate(&context, value);

        assert!(result.is_ok());
        assert!(!context.has_errors());
    }

    #[test]
    fn test_validate_one_of_with_array_of_schemas() {
        let root_schema = loader::load_from_str(
            r##"
            $defs:
              schema:
                type: object
                properties:
                  type:
                    enum: [string, object, number, integer, boolean, enum, array, oneOf, anyOf, not]
              array_of_schemas:
                type: array
                items:
                  $ref: "#/$defs/schema"
            oneOf:
              - type: boolean
              - $ref: "#/$defs/array_of_schemas"
            "##,
        )
        .expect("Failed to load schema");
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        if let Some(_one_of) = &subschema.one_of {
            // oneOf schema loaded successfully
        } else {
            panic!("Expected Subschema with oneOf, but got: {subschema:?}");
        }

        let s = r#"
            false
            "#;
        let docs = MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::with_root_schema(&root_schema, false);
        let result = root_schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(!context.has_errors());
        assert!(!context.has_errors());
    }

    #[test]
    fn test_validate_one_of_with_null_and_object() {
        let root_schema = loader::load_from_str(
            r#"
            oneOf:
              - type: null
              - type: object
            "#,
        )
        .expect("Failed to load schema");

        let s = "null";
        let docs = MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::with_root_schema(&root_schema, false);
        let result = root_schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(!context.has_errors());

        let s = r#"
        name: "John Doe"
        "#;
        let docs = MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::with_root_schema(&root_schema, false);
        let result = root_schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(!context.has_errors());
    }
}
