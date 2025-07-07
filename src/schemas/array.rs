use super::{BoolOrTypedSchema, TypedSchema};
use crate::loader::{FromAnnotatedMapping, FromSaphyrMapping};
use crate::utils::format_yaml_data;
use crate::utils::{format_marker, format_vec};
use crate::Validator;
use crate::YamlSchema;
use crate::{loader, Result};
use crate::{Context, Reference};
use log::debug;
use saphyr::{AnnotatedMapping, MarkedYaml, Scalar, YamlData};

/// An array schema represents an array
#[derive(Debug, Default, PartialEq)]
pub struct ArraySchema {
    pub items: Option<BoolOrTypedSchema>,
    pub prefix_items: Option<Vec<YamlSchema>>,
    pub contains: Option<Box<YamlSchema>>,
}

impl ArraySchema {
    pub fn with_items_typed(typed_schema: TypedSchema) -> Self {
        Self {
            items: Some(BoolOrTypedSchema::TypedSchema(Box::new(typed_schema))),
            ..Default::default()
        }
    }

    pub fn with_items_ref(reference: Reference) -> Self {
        Self {
            items: Some(BoolOrTypedSchema::Reference(reference)),
            ..Default::default()
        }
    }
}

impl FromSaphyrMapping<ArraySchema> for ArraySchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> Result<ArraySchema> {
        let mut array_schema = ArraySchema::default();
        for (key, value) in mapping.iter() {
            let s = loader::load_string_value(key)?;
            match s.as_str() {
                "contains" => {
                    if let saphyr::Yaml::Mapping(mapping) = value {
                        let yaml_schema = YamlSchema::from_mapping(mapping)?;
                        array_schema.contains = Some(Box::new(yaml_schema));
                    } else {
                        return Err(generic_error!(
                            "contains: expected a mapping, but got: {:#?}",
                            value
                        ));
                    }
                }
                "items" => {
                    let array_items = loader::load_array_items(value)?;
                    array_schema.items = Some(array_items);
                }
                "type" => {
                    let s = loader::load_string_value(value)?;
                    if s != "array" {
                        return Err(unsupported_type!("Expected type: array, but got: {}", s));
                    }
                }
                "prefixItems" => {
                    let prefix_items = loader::load_array_of_schemas(value)?;
                    array_schema.prefix_items = Some(prefix_items);
                }
                _ => unimplemented!("Unsupported key for ArraySchema: {}", s),
            }
        }
        Ok(array_schema)
    }
}

impl FromAnnotatedMapping<ArraySchema> for ArraySchema {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> Result<ArraySchema> {
        let mut array_schema = ArraySchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(s)) = &key.data {
                match s.as_ref() {
                    "contains" => {
                        if value.data.is_mapping() {
                            let yaml_schema = value.try_into()?;
                            array_schema.contains = Some(Box::new(yaml_schema));
                        } else {
                            return Err(generic_error!(
                                "contains: expected a mapping, but got: {:#?}",
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
                            return Err(generic_error!(
                                "{} Expected string value for `type:`, got {:?}",
                                format_marker(&value.span.start),
                                value
                            ));
                        }
                    }
                    "prefixItems" => {
                        let prefix_items = loader::load_array_of_schemas_marked(value)?;
                        array_schema.prefix_items = Some(prefix_items);
                    }
                    _ => unimplemented!("Unsupported key for ArraySchema: {}", s),
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

impl Validator for ArraySchema {
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
                    sub_schema.validate(&sub_context, item).is_ok()
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
                            BoolOrTypedSchema::Boolean(true) => {
                                // `items: true` allows any items
                                break;
                            }
                            BoolOrTypedSchema::Boolean(false) => {
                                context.add_error(
                                    item,
                                    "Additional array items are not allowed!".to_string(),
                                );
                            }
                            BoolOrTypedSchema::TypedSchema(typed_schema) => {
                                typed_schema.validate(context, item)?;
                            }
                            BoolOrTypedSchema::Reference(reference) => {
                                // Grab the reference from the root schema.
                                let Some(root) = &context.root_schema else {
                                    context.add_error(
                                        item,
                                        "No root schema was provided to look up references"
                                            .to_string(),
                                    );
                                    continue;
                                };
                                let Some(def) = root.get_def(&reference.ref_name) else {
                                    context.add_error(
                                        item,
                                        format!("No definition for {} found", reference.ref_name),
                                    );
                                    continue;
                                };
                                def.validate(context, item)?;
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
                        BoolOrTypedSchema::Boolean(true) => { /* no-op */ }
                        BoolOrTypedSchema::Boolean(false) => {
                            if self.prefix_items.is_none() && !array.is_empty() {
                                context.add_error(
                                    array.first().unwrap(),
                                    "Array items are not allowed!".to_string(),
                                );
                            }
                        }
                        BoolOrTypedSchema::TypedSchema(typed_schema) => {
                            for item in array {
                                typed_schema.validate(context, item)?;
                            }
                        }
                        BoolOrTypedSchema::Reference(reference) => {
                            // Grab the reference from the root schema.
                            let Some(root) = &context.root_schema else {
                                context.add_error(
                                    array.first().unwrap(),
                                    "No root schema was provided to look up references".to_string(),
                                );
                                return Ok(());
                            };
                            let Some(def) = root.get_def(&reference.ref_name) else {
                                context.add_error(
                                    array.first().unwrap(),
                                    format!("No definition for {} found", reference.ref_name),
                                );
                                return Ok(());
                            };
                            for item in array {
                                def.validate(context, item)?;
                            }
                        }
                    }
                }
            }

            Ok(())
        } else {
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

impl std::fmt::Display for ArraySchema {
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
    use crate::loader::FromSaphyrMapping;
    use crate::NumberSchema;
    use crate::Schema;
    use crate::StringSchema;
    use crate::TypedSchema;
    use saphyr::LoadableYamlNode;

    use super::*;

    #[test]
    fn test_array_schema_prefix_items() {
        let schema = ArraySchema {
            prefix_items: Some(vec![YamlSchema::from(Schema::Number(
                NumberSchema::default(),
            ))]),
            items: Some(BoolOrTypedSchema::TypedSchema(Box::new(
                TypedSchema::String(StringSchema::default()),
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

        let s_docs = saphyr::Yaml::load_from_str(schema_string).unwrap();
        let first_schema = s_docs.first().unwrap();
        if let saphyr::Yaml::Mapping(array_schema_hash) = first_schema {
            let schema = ArraySchema::from_mapping(array_schema_hash).unwrap();
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
            let schema: ArraySchema = ArraySchema::from_annotated_mapping(mapping).unwrap();
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
        let number_schema = YamlSchema::from(Schema::Number(NumberSchema::default()));
        let schema = ArraySchema {
            contains: Some(Box::new(number_schema)),
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
    fn test_contains_fails() {
        let number_schema = YamlSchema::from(Schema::Number(NumberSchema::default()));
        let schema = ArraySchema {
            contains: Some(Box::new(number_schema)),
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
