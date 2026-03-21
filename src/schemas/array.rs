use std::collections::HashSet;
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
pub struct ArraySchema {
    pub items: Option<BooleanOrSchema>,
    pub prefix_items: Option<Vec<YamlSchema>>,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub unique_items: Option<bool>,
    pub contains: Option<YamlSchema>,
    pub min_contains: Option<u64>,
    pub max_contains: Option<u64>,
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for ArraySchema {
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
                    "minContains" => {
                        let n = loader::load_integer_marked(value)?;
                        if n < 0 {
                            return Err(generic_error!(
                                "{} minContains must be a non-negative integer, got: {}",
                                format_marker(&value.span.start),
                                n
                            ));
                        }
                        array_schema.min_contains = Some(n as u64);
                    }
                    "maxContains" => {
                        let n = loader::load_integer_marked(value)?;
                        if n < 0 {
                            return Err(generic_error!(
                                "{} maxContains must be a non-negative integer, got: {}",
                                format_marker(&value.span.start),
                                n
                            ));
                        }
                        array_schema.max_contains = Some(n as u64);
                    }
                    "minItems" => {
                        if let Ok(i) = loader::load_integer_marked(value) {
                            array_schema.min_items = Some(i as usize);
                        } else {
                            return Err(unsupported_type!(
                                "minItems expected integer, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "maxItems" => {
                        if let Ok(i) = loader::load_integer_marked(value) {
                            array_schema.max_items = Some(i as usize);
                        } else {
                            return Err(unsupported_type!(
                                "maxItems expected integer, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "uniqueItems" => {
                        if let YamlData::Value(Scalar::Boolean(b)) = &value.data {
                            array_schema.unique_items = Some(*b);
                        } else {
                            return Err(unsupported_type!(
                                "uniqueItems expected boolean, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "unevaluatedItems" => {
                        // Loaded on `Subschema`; ignore here when parsing `type: array` mapping.
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

impl Validator for ArraySchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[ArraySchema] self: {self:?}");
        let data = &value.data;
        debug!("[ArraySchema] Validating value: {}", format_yaml_data(data));

        if let saphyr::YamlData::Sequence(array) = data {
            let err_after_meta = context.errors.borrow().len();

            // validate contains with minContains / maxContains
            if let Some(min_items) = self.min_items
                && array.len() < min_items
            {
                context.add_error(
                    value,
                    format!(
                        "Array has too few items (minimum {min_items}, found {})",
                        array.len()
                    ),
                );
                fail_fast!(context);
            }
            if let Some(max_items) = self.max_items
                && array.len() > max_items
            {
                context.add_error(
                    value,
                    format!(
                        "Array has too many items (maximum {max_items}, found {})",
                        array.len()
                    ),
                );
                fail_fast!(context);
            }

            if self.unique_items == Some(true) {
                let mut seen = HashSet::with_capacity(array.len());
                for item in array {
                    if !seen.insert(item) {
                        context.add_error(
                            item,
                            format!("Duplicate array element: {}", format_yaml_data(&item.data)),
                        );
                        fail_fast!(context);
                    }
                }
            }

            // validate contains
            if let Some(sub_schema) = &self.contains {
                let match_count = array
                    .iter()
                    .filter(|item| {
                        let sub_context = crate::Context {
                            root_schema: context.root_schema,
                            fail_fast: true,
                            ..Default::default()
                        };
                        sub_schema.validate(&sub_context, item).is_ok() && !sub_context.has_errors()
                    })
                    .count() as u64;

                let min = self.min_contains.unwrap_or(1);
                if match_count < min {
                    context.add_error(
                        value,
                        format!(
                            "Array must contain at least {min} item(s) matching the contains schema, but only {match_count} matched"
                        ),
                    );
                }
                if let Some(max) = self.max_contains
                    && match_count > max
                {
                    context.add_error(
                        value,
                        format!(
                            "Array must contain at most {max} item(s) matching the contains schema, but {match_count} matched"
                        ),
                    );
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

            if context.errors.borrow().len() == err_after_meta {
                Self::record_unevaluated_array_annotations(self, context, array);
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

impl ArraySchema {
    /// Update [`Context::array_unevaluated`] from this schema's `prefixItems` / `items` / `contains` (2020-12).
    fn record_unevaluated_array_annotations(
        schema: &ArraySchema,
        context: &Context,
        array: &[MarkedYaml],
    ) {
        let Some(cell) = context.array_unevaluated.as_ref() else {
            return;
        };
        let mut ann = cell.borrow_mut();

        if let Some(sub_schema) = &schema.contains {
            ann.saw_relevant = true;
            if array.is_empty() {
                // Annotation still present for empty instance (Core §10.3.1.3).
            } else {
                let mut matching = HashSet::new();
                for (i, item) in array.iter().enumerate() {
                    let sub_context = Context {
                        root_schema: context.root_schema,
                        fail_fast: true,
                        ..Default::default()
                    };
                    if sub_schema.validate(&sub_context, item).is_ok() && !sub_context.has_errors()
                    {
                        matching.insert(i);
                    }
                }
                if matching.len() == array.len() {
                    ann.contains_all = true;
                } else {
                    ann.contains_indices.extend(matching);
                }
            }
        }

        if let Some(prefix_items) = &schema.prefix_items
            && !prefix_items.is_empty()
            && !array.is_empty()
        {
            let n = array.len().min(prefix_items.len());
            if n > 0 {
                ann.saw_relevant = true;
                let largest = n - 1;
                ann.prefix_largest = Some(match ann.prefix_largest {
                    Some(p) => p.max(largest),
                    None => largest,
                });
            }
        }

        let prefix_len = schema.prefix_items.as_ref().map(|p| p.len()).unwrap_or(0);
        let tail_non_empty = array.len() > prefix_len;
        let items_covers_all = prefix_len == 0 && !array.is_empty();

        if let Some(items) = &schema.items {
            match items {
                BooleanOrSchema::Boolean(true) => {
                    if tail_non_empty || items_covers_all {
                        ann.saw_relevant = true;
                        ann.full_coverage = true;
                    }
                }
                BooleanOrSchema::Schema(_) => {
                    if tail_non_empty || items_covers_all {
                        ann.saw_relevant = true;
                        ann.full_coverage = true;
                    }
                }
                BooleanOrSchema::Boolean(false) => {}
            }
        }
    }
}

impl Display for ArraySchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Array{{ items: {:?}, prefix_items: {:?}, min_items: {:?}, max_items: {:?}, unique_items: {:?}}}, contains: {:?}, min_contains: {:?}, max_contains: {:?}}}",
            self.items,
            self.prefix_items,
            self.min_items,
            self.max_items,
            self.unique_items,
            self.contains,
            self.min_contains,
            self.max_contains
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
        assert!(result.is_ok());
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
            assert!(result.is_ok());
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
            assert!(result.is_ok());
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
    fn test_min_items_valid() {
        let schema = ArraySchema {
            min_items: Some(2),
            ..Default::default()
        };
        let s = "- 1\n- 2\n- 3";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        schema.validate(&context, value).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_min_items_invalid() {
        let schema = ArraySchema {
            min_items: Some(3),
            ..Default::default()
        };
        let s = "- 1\n- 2";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        schema.validate(&context, value).unwrap();
        assert!(context.has_errors());
    }

    #[test]
    fn test_max_items_valid() {
        let schema = ArraySchema {
            max_items: Some(3),
            ..Default::default()
        };
        let s = "- 1\n- 2";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        schema.validate(&context, value).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_max_items_invalid() {
        let schema = ArraySchema {
            max_items: Some(2),
            ..Default::default()
        };
        let s = "- 1\n- 2\n- 3";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        schema.validate(&context, value).unwrap();
        assert!(context.has_errors());
    }

    #[test]
    fn test_min_items_from_yaml() {
        let schema_string = "type: array\nminItems: 2";
        let s_docs = saphyr::MarkedYaml::load_from_str(schema_string).unwrap();
        let first_schema = s_docs.first().unwrap();
        if let YamlData::Mapping(mapping) = &first_schema.data {
            let schema = ArraySchema::try_from(mapping).unwrap();
            assert_eq!(schema.min_items, Some(2));
        } else {
            panic!("Expected mapping");
        }
    }

    #[test]
    fn test_max_items_from_yaml() {
        let schema_string = "type: array\nmaxItems: 5";
        let s_docs = saphyr::MarkedYaml::load_from_str(schema_string).unwrap();
        let first_schema = s_docs.first().unwrap();
        if let YamlData::Mapping(mapping) = &first_schema.data {
            let schema = ArraySchema::try_from(mapping).unwrap();
            assert_eq!(schema.max_items, Some(5));
        } else {
            panic!("Expected mapping");
        }
    }

    #[test]
    fn test_unique_items_valid() {
        let schema = ArraySchema {
            unique_items: Some(true),
            ..Default::default()
        };
        let s = "- 1\n- 2\n- 3";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        schema.validate(&context, value).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_unique_items_invalid() {
        let schema = ArraySchema {
            unique_items: Some(true),
            ..Default::default()
        };
        let s = "- 1\n- 2\n- 1";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        schema.validate(&context, value).unwrap();
        assert!(context.has_errors());
    }

    #[test]
    fn test_unique_items_false_allows_duplicates() {
        let schema = ArraySchema {
            unique_items: Some(false),
            ..Default::default()
        };
        let s = "- 1\n- 1\n- 2";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        schema.validate(&context, value).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_unique_items_empty_array() {
        let schema = ArraySchema {
            unique_items: Some(true),
            ..Default::default()
        };
        let s = "[]";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::default();
        schema.validate(&context, value).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_unique_items_from_yaml() {
        let schema_string = "type: array\nuniqueItems: true";
        let s_docs = saphyr::MarkedYaml::load_from_str(schema_string).unwrap();
        let first_schema = s_docs.first().unwrap();
        if let YamlData::Mapping(mapping) = &first_schema.data {
            let schema = ArraySchema::try_from(mapping).unwrap();
            assert_eq!(schema.unique_items, Some(true));
        } else {
            panic!("Expected mapping");
        }
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

    #[test]
    fn test_min_contains() {
        let number_schema = YamlSchema::typed_number(NumberSchema::default());
        let schema = ArraySchema {
            contains: Some(number_schema),
            min_contains: Some(2),
            ..Default::default()
        };

        // 2 numbers — passes
        let s = "- apple\n- 1\n- 2\n";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let context = crate::Context::default();
        schema.validate(&context, docs.first().unwrap()).unwrap();
        assert!(context.errors.take().is_empty());

        // only 1 number — fails
        let s = "- apple\n- 1\n- banana\n";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let context = crate::Context::default();
        schema.validate(&context, docs.first().unwrap()).unwrap();
        assert!(!context.errors.take().is_empty());
    }

    #[test]
    fn test_max_contains() {
        let number_schema = YamlSchema::typed_number(NumberSchema::default());
        let schema = ArraySchema {
            contains: Some(number_schema),
            max_contains: Some(2),
            ..Default::default()
        };

        // 2 numbers — passes
        let s = "- 1\n- apple\n- 2\n";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let context = crate::Context::default();
        schema.validate(&context, docs.first().unwrap()).unwrap();
        assert!(context.errors.take().is_empty());

        // 3 numbers — fails
        let s = "- 1\n- 2\n- 3\n";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let context = crate::Context::default();
        schema.validate(&context, docs.first().unwrap()).unwrap();
        assert!(!context.errors.take().is_empty());
    }

    #[test]
    fn test_min_contains_zero() {
        let number_schema = YamlSchema::typed_number(NumberSchema::default());
        let schema = ArraySchema {
            contains: Some(number_schema),
            min_contains: Some(0),
            ..Default::default()
        };

        // no numbers — still passes because minContains is 0
        let s = "- apple\n- banana\n";
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let context = crate::Context::default();
        schema.validate(&context, docs.first().unwrap()).unwrap();
        assert!(context.errors.take().is_empty());
    }
}
