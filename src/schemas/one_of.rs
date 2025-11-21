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

/// The `oneOf` schema is a schema that matches if one, and only one of the schemas in the `oneOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
#[derive(Debug, Default, PartialEq)]
pub struct OneOfSchema {
    pub one_of: Vec<YamlSchema>,
}

impl std::fmt::Display for OneOfSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "oneOf:{}", format_vec(&self.one_of))
    }
}

impl TryFrom<&MarkedYaml<'_>> for OneOfSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<Self> {
        if let YamlData::Mapping(mapping) = &value.data {
            OneOfSchema::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for OneOfSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> Result<Self> {
        match mapping.get(&MarkedYaml::value_from_str("oneOf")) {
            Some(value) => {
                let one_of = loader::load_array_of_schemas_marked(value)?;
                Ok(OneOfSchema { one_of })
            }
            None => Err(generic_error!("No `oneOf` key found!")),
        }
    }
}

impl Validator for crate::schemas::OneOfSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let one_of_is_valid = validate_one_of(context, &self.one_of, value)?;
        if !one_of_is_valid {
            error!("OneOf: None of the schemas in `oneOf` matched!");
            context.add_error(value, "None of the schemas in `oneOf` matched!");
            fail_fast!(context);
        }
        Ok(())
    }
}

pub fn validate_one_of(
    context: &Context,
    schemas: &Vec<YamlSchema>,
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
    use crate::RootSchema;
    use crate::Schema;
    use saphyr::LoadableYamlNode;

    #[test]
    fn test_validate_one_of_with_array_of_schemas() {
        let root_schema = RootSchema::load_from_str(
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
        .unwrap();
        println!("root_schema: {root_schema:#?}");
        let root_schema_schema = root_schema.schema.as_ref().schema.as_ref().unwrap();
        if let Schema::OneOf(one_of_schema) = root_schema_schema {
            println!("one_of_schema: {one_of_schema:#?}");
        } else {
            panic!("Expected Schema::OneOf, but got: {root_schema_schema:?}");
        }

        let s = r#"
            false
            "#;
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::with_root_schema(&root_schema, false);
        let result = root_schema.validate(&context, value);
        println!("result: {result:#?}");
        assert!(result.is_ok());
        for error in context.errors.borrow().iter() {
            println!("error: {error:#?}");
        }
        assert!(!context.has_errors());
    }
}
