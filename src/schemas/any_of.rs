use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::YamlSchema;
use crate::loader;
use crate::loader::FromAnnotatedMapping;
use crate::utils::format_vec;

/// The `anyOf` schema is a schema that matches if any of the schemas in the `anyOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
#[derive(Debug, Default, PartialEq)]
pub struct AnyOfSchema {
    pub any_of: Vec<YamlSchema>,
}

impl std::fmt::Display for AnyOfSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "anyOf:{}", format_vec(&self.any_of))
    }
}

impl FromAnnotatedMapping<AnyOfSchema> for AnyOfSchema {
    fn from_annotated_mapping(
        mapping: &AnnotatedMapping<MarkedYaml>,
    ) -> crate::Result<AnyOfSchema> {
        let mut any_of_schema = AnyOfSchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(key)) = &key.data {
                match key.as_ref() {
                    "anyOf" => {
                        any_of_schema.any_of = loader::load_array_of_schemas_marked(value)?;
                    }
                    _ => return Err(generic_error!("Unsupported key: {}", key)),
                }
            }
        }
        Ok(any_of_schema)
    }
}
