// The codegen module is responsible for generating Rust code that uses the yaml-schema library

use crate::Result;
use crate::RootSchema;
use log::debug;

pub fn generate_code_from_root_schema(root_schema: &RootSchema) -> Result<String> {
    debug!("Generating code from root schema");
    Ok("".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Schema::Object;
    use crate::{
        ArraySchema, ObjectSchema, OneOfSchema, Reference, Schema, StringSchema, YamlSchema,
    };
    use hashlink::LinkedHashMap;

    use crate::BoolOrTypedSchema::TypedSchema;
    use saphyr::Yaml;
    use std::collections::HashMap;
    use std::fs::metadata;
    use std::hash::Hash;
    use std::rc::Rc;

    fn vec_of_string(items: Vec<&str>) -> Vec<String> {
        items.into_iter().map(|s| s.to_string()).collect()
    }

    fn vec_to_linked_hash_map<K, V>(items: Vec<(K, V)>) -> LinkedHashMap<K, V>
    where
        K: Hash + Eq + Clone,
    {
        items.into_iter().collect()
    }

    fn vec_to_hash_map<K, V>(items: Vec<(K, V)>) -> HashMap<K, V>
    where
        K: Hash + Eq + Clone,
    {
        items.into_iter().collect()
    }

    fn schema_type_def() -> YamlSchema {
        YamlSchema {
            metadata: Some(vec_to_linked_hash_map(vec![(
                "description".to_string(),
                "The type of the schema".to_string(),
            )])),
            r#ref: None,
            schema: Some(Schema::String(
                StringSchema::builder()
                    .r#enum(vec_of_string(vec![
                        "object", "string", "number", "integer", "boolean", "enum", "array",
                        "oneOF", "anyOf", "not",
                    ]))
                    .build(),
            )),
        }
    }

    fn schema_def() -> YamlSchema {
        let object_schema = ObjectSchema::builder()
            .property("type", YamlSchema::reference("schema_type"))
            .property(
                "properties",
                YamlSchema::builder()
                    .metadata(
                        "description",
                        "The properties that are defined in the schema",
                    )
                    .schema(pattern_property_schema_def())
                    .build(),
            )
            .property("description", YamlSchema::string())
            .build();
        YamlSchema::builder()
            .metadata("description", "A meta schema for a YAML object schema")
            .schema(Schema::object(object_schema))
            .build()
    }

    fn array_of_schemas_def() -> YamlSchema {
        YamlSchema::builder()
            .metadata("description", "An array of schemas")
            .schema(Schema::Array(ArraySchema::with_items_ref(Reference::new(
                "schema",
            ))))
            .build()
    }

    fn pattern_property_schema_def() -> Schema {
        Schema::object(
            ObjectSchema::builder()
                .pattern_property("^[a-zA-Z0-9_-]+$", YamlSchema::reference("schema"))
                .build(),
        )
    }

    #[test]
    #[ignore]
    fn test_generate_code_from_root_schema() -> Result<()> {
        let expected = RootSchema::load_file("yaml-schema.yaml")?;
        debug!("{expected:#?}");

        let defs: LinkedHashMap<String, YamlSchema> = vec![
            ("schema_type", schema_type_def()),
            ("schema", schema_def()),
            ("array_of_schemas", array_of_schemas_def()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

        let dollar_schema_schema = YamlSchema::builder()
            .description("Specifies which draft of the JSON Schema standard the schema adheres to.")
            .string_schema(StringSchema::default())
            .build();
        let dollar_id_schema = YamlSchema::builder()
            .description("Sets a URI for the schema. You can use this unique URI to refer to elements of the schema from inside the same document or from external JSON documents.")
            .string_schema(StringSchema::default())
            .build();
        let dollar_defs_schema = YamlSchema::builder()
            .description("A container for reusable JSON Schema fragments.")
            .schema(pattern_property_schema_def())
            .build();
        let additional_properties_schema = YamlSchema::builder()
            .schema(Schema::OneOf(OneOfSchema {
                one_of: vec![
                    Schema::BooleanSchema.into(),
                    YamlSchema::reference("array_of_schemas"),
                ],
            }))
            .build();
        let schema = YamlSchema::object(
            ObjectSchema::builder()
                .property("$schema", dollar_schema_schema)
                .property("$id", dollar_id_schema)
                .property("$defs", dollar_defs_schema)
                .property(
                    "title",
                    YamlSchema::builder()
                        .description("The title of the schema")
                        .string_schema(StringSchema::default())
                        .build(),
                )
                .property(
                    "description",
                    YamlSchema::builder()
                        .description("A description of the schema")
                        .string_schema(StringSchema::default())
                        .build(),
                )
                .property(
                    "type",
                    YamlSchema::builder()
                        .description("defines the first constraint on the JSON data.")
                        .r#ref(Reference::new("schema_type"))
                        .build(),
                )
                .property(
                    "properties",
                    YamlSchema::builder()
                        .description("The properties that are defined in the schema")
                        .schema(pattern_property_schema_def())
                        .build(),
                )
                .property("additionalProperties", additional_properties_schema)
                .additional_properties(false)
                .build(),
        );
        let actual = RootSchema::builder()
            .id("https://yaml-schema.net/draft/2020-12/meta-schema")
            .meta_schema("https://yaml-schema.net/draft/2020-12/schema")
            .defs(defs)
            .schema(schema)
            .build();

        assert_eq!(expected, actual);

        let code = generate_code_from_root_schema(&expected)?;
        println!("{code}");

        Ok(())
    }
}
