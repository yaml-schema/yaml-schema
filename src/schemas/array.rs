use std::fmt::Display;

use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::Context;
use crate::Result;
use crate::Validator;
use crate::YamlSchema;
use crate::loader;
use crate::schemas::BooleanOrSchema;
use crate::utils::format_marker;
use crate::utils::format_vec;
use crate::utils::format_yaml_data;

/// An array schema represents an array
#[derive(Debug, Default, PartialEq)]
pub struct ArraySchema<'r> {
    pub items: Option<BooleanOrSchema<'r>>,
    pub prefix_items: Option<Vec<YamlSchema<'r>>>,
    pub contains: Option<YamlSchema<'r>>,
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for ArraySchema<'r> {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'r, MarkedYaml<'r>>) -> crate::Result<Self> {
        let mut array_schema = ArraySchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(s)) = &key.data {
                match s.as_ref() {
                    "contains" => {
                        if value.data.is_mapping() {
                            let yaml_schema = value.try_into()?;
                            array_schema.contains = Some(yaml_schema);
                        } else {
                            return Err(generic_error!(
                                "contains: expected a mapping, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "items" => {
                        let array_items = loader::load_array_items_marked(value)?;
                        array_schema.items = Some(array_items);
                    }
                    "type" => {
                        if let YamlData::Value(Scalar::String(s)) = &value.data {
                            if s != "array" {
                                return Err(unsupported_type!(
                                    "Expected type: array, but got: {}",
                                    s
                                ));
                            }
                        } else {
                            return Err(expected_type_is_string!(value));
                        }
                    }
                    "prefixItems" => {
                        let prefix_items = loader::load_array_of_schemas_marked(value)?;
                        array_schema.prefix_items = Some(prefix_items);
                    }
                    _ => debug!("Unsupported key for ArraySchema: {}", s),
                }
            } else {
                return Err(generic_error!(
                    "{} Expected scalar key, got: {:?}",
                    format_marker(&key.span.start),
                    key
                ));
            }
        }
        Ok(array_schema)
    }
}

impl Validator for ArraySchema<'_> {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[ArraySchema] self: {self:?}");
        let data = &value.data;
        debug!("[ArraySchema] Validating value: {}", format_yaml_data(data));

        if let saphyr::YamlData::Sequence(array) = data {
            // validate contains
            if let Some(sub_schema) = &self.contains {
                let any_matches = array.iter().any(|item| {
                    let sub_context = crate::Context {
                        root_schema: context.root_schema,
                        fail_fast: true,
                        ..Default::default()
                    };
                    sub_schema.validate(&sub_context, item).is_ok() && !sub_context.has_errors()
                });
                if !any_matches {
                    context.add_error(value, "Contains validation failed!".to_string());
                }
            }

            // validate prefix items
            if let Some(prefix_items) = &self.prefix_items {
                debug!(
                    "[ArraySchema] Validating prefix items: {}",
                    format_vec(prefix_items)
                );
                for (i, item) in array.iter().enumerate() {
                    // if the index is within the prefix items, validate against the prefix items schema
                    if i < prefix_items.len() {
                        debug!(
                            "[ArraySchema] Validating prefix item {} with schema: {}",
                            i, prefix_items[i]
                        );
                        prefix_items[i].validate(context, item)?;
                    } else if let Some(items) = &self.items {
                        // if the index is not within the prefix items, validate against the array items schema
                        debug!("[ArraySchema] Validating array item {i} with schema: {items}");
                        match items {
                            BooleanOrSchema::Boolean(true) => {
                                // `items: true` allows any items
                                break;
                            }
                            BooleanOrSchema::Boolean(false) => {
                                context.add_error(
                                    item,
                                    "Additional array items are not allowed!".to_string(),
                                );
                            }
                            BooleanOrSchema::Schema(yaml_schema) => {
                                yaml_schema.validate(context, item)?;
                            }
                        }
                    } else {
                        break;
                    }
                }
            } else {
                // validate array items
                if let Some(items) = &self.items {
                    match items {
                        BooleanOrSchema::Boolean(true) => { /* no-op */ }
                        BooleanOrSchema::Boolean(false) => {
                            if self.prefix_items.is_none() && !array.is_empty() {
                                context
                                    .add_error(value, "Array items are not allowed!".to_string());
                            }
                        }
                        BooleanOrSchema::Schema(yaml_schema) => {
                            for item in array {
                                yaml_schema.validate(context, item)?;
                            }
                        }
                    }
                }
            }

            Ok(())
        } else {
            debug!("[ArraySchema] context.fail_fast: {}", context.fail_fast);
            context.add_error(
                value,
                format!(
                    "Expected an array, but got: {}",
                    format_yaml_data(&value.data)
                ),
            );
            fail_fast!(context);
            Ok(())
        }
    }
}

impl Display for ArraySchema<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Array{{ items: {:?}, prefix_items: {:?}, contains: {:?}}}",
            self.items, self.prefix_items, self.contains
        )
    }
}
#[cfg(test)]
mod tests {
    use crate::schemas::NumberSchema;
    use crate::schemas::StringSchema;
    use saphyr::LoadableYamlNode;

    use super::*;

    #[test]
    fn test_array_schema_prefix_items() {
        let schema = ArraySchema {
            prefix_items: Some(vec![YamlSchema::typed_number(NumberSchema::default())]),
            items: Some(BooleanOrSchema::schema(YamlSchema::typed_string(
                StringSchema::default(),
            ))),
            ..Default::default()
        };
        let s = r#"
        - 1
        - 2
        - Washington
        "#;
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        let result = schema.validate(&context, value);
        if result.is_err() {
            println!("{}", result.unwrap_err());
        }
    }

    #[test]
    fn test_array_schema_prefix_items_from_yaml() {
        let schema_string = "
      type: array
      prefixItems:
        - type: number
        - type: string
        - enum:
          - Street
          - Avenue
          - Boulevard
        - enum:
          - NW
          - NE
          - SW
          - SE
      items:
        type: string
";

        let yaml_string = r#"
        - 1600
        - Pennsylvania
        - Avenue
        - NW
        - Washington
        "#;

        let s_docs = saphyr::MarkedYaml::load_from_str(schema_string).unwrap();
        let first_schema = s_docs.first().unwrap();
        if let YamlData::Mapping(mapping) = &first_schema.data {
            let schema = ArraySchema::try_from(mapping).unwrap();
            let docs = saphyr::MarkedYaml::load_from_str(yaml_string).unwrap();
            let value = docs.first().unwrap();
            let context = crate::Context::default();
            let result = schema.validate(&context, value);
            if result.is_err() {
                println!("{}", result.unwrap_err());
            }
        } else {
            panic!("Expected first_schema to be a Mapping, but got {first_schema:?}");
        }
    }

    #[test]
    fn array_schema_prefix_items_with_additional_items() {
        let schema_string = "
      type: array
      prefixItems:
        - type: number
        - type: string
        - enum:
          - Street
          - Avenue
          - Boulevard
        - enum:
          - NW
          - NE
          - SW
          - SE
      items:
        type: string
";

        let yaml_string = r#"
        - 1600
        - Pennsylvania
        - Avenue
        - NW
        - 20500
        "#;

        let docs = MarkedYaml::load_from_str(schema_string).unwrap();
        let first_doc = docs.first().unwrap();
        if let YamlData::Mapping(mapping) = &first_doc.data {
            let schema: ArraySchema = ArraySchema::try_from(mapping).unwrap();
            let docs = saphyr::MarkedYaml::load_from_str(yaml_string).unwrap();
            let value = docs.first().unwrap();
            let context = crate::Context::default();
            let result = schema.validate(&context, value);
            if result.is_err() {
                println!("{}", result.unwrap_err());
            }
        } else {
            panic!("Expected first_doc to be a Mapping, but got {first_doc:?}");
        }
    }

    #[test]
    fn test_contains() {
        let number_schema = YamlSchema::typed_number(NumberSchema::default());
        let schema = ArraySchema {
            contains: Some(number_schema),
            ..Default::default()
        };
        let s = r#"
        - life
        - universe
        - everything
        - 42
        "#;
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        let result = schema.validate(&context, value);
        assert!(result.is_ok());
        let errors = context.errors.take();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_array_schema_contains_fails() {
        let number_schema = YamlSchema::typed_number(NumberSchema::default());
        let schema = ArraySchema {
            contains: Some(number_schema),
            ..Default::default()
        };
        let s = r#"
        - life
        - universe
        - everything
        "#;
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        let result = schema.validate(&context, value);
        assert!(result.is_ok());
        let errors = context.errors.take();
        assert!(!errors.is_empty());
    }
}
