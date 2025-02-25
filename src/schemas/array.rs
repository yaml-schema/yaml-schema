use log::debug;

use crate::format_vec;
use crate::format_yaml_data;
use crate::Context;
use crate::Result;
use crate::Validator;
use crate::YamlSchema;

use super::BoolOrTypedSchema;

/// An array schema represents an array
#[derive(Debug, Default, PartialEq)]
pub struct ArraySchema {
    pub items: Option<BoolOrTypedSchema>,
    pub prefix_items: Option<Vec<YamlSchema>>,
    pub contains: Option<Box<YamlSchema>>,
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

impl Validator for ArraySchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[ArraySchema] self: {:?}", self);
        let data = &value.data;
        debug!("[ArraySchema] Validating value: {}", format_yaml_data(data));

        if !data.is_array() {
            context.add_error(
                value,
                format!(
                    "Expected an array, but got: {}",
                    format_yaml_data(&value.data)
                ),
            );
            fail_fast!(context);
            return Ok(());
        }

        let array = data.as_vec().unwrap();

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
                    debug!(
                        "[ArraySchema] Validating array item {} with schema: {}",
                        i, items
                    );
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
                                    "No root schema was provided to look up references".to_string(),
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
    }
}

#[cfg(test)]
mod tests {
    use crate::loader::Constructor;
    use crate::NumberSchema;
    use crate::Schema;
    use crate::StringSchema;
    use crate::TypedSchema;

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
        let schema_string = r#"      type: array
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
        "#;

        let yaml_string = r#"
        - 1600
        - Pennsylvania
        - Avenue
        - NW
        - Washington
        "#;

        let s_docs = saphyr::Yaml::load_from_str(schema_string).unwrap();
        let first_schema = s_docs.first().unwrap();
        let array_schema_hash = first_schema.as_hash().unwrap();
        let schema = ArraySchema::construct(array_schema_hash).unwrap();
        let docs = saphyr::MarkedYaml::load_from_str(yaml_string).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        let result = schema.validate(&context, value);
        if result.is_err() {
            println!("{}", result.unwrap_err());
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
