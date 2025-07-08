use crate::loader;
use crate::loader::{FromAnnotatedMapping, FromSaphyrMapping};
/// The `oneOf` schema is a schema that matches if one, and only one of the schemas in the `oneOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
use crate::utils::{format_marker, format_vec};
use crate::YamlSchema;
use saphyr::{AnnotatedMapping, MarkedYaml, YamlData};

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

    fn try_from(value: &MarkedYaml) -> Result<Self, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
            Self::from_annotated_mapping(mapping)
        } else {
            Err(generic_error!(
                "{} expected mapping, got {:?}",
                format_marker(&value.span.start),
                value
            ))
        }
    }
}

impl FromSaphyrMapping<OneOfSchema> for OneOfSchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> crate::Result<OneOfSchema> {
        let mut one_of_schema = OneOfSchema::default();
        for (key, value) in mapping.iter() {
            if let Ok(key) = loader::load_string_value(key) {
                match key.as_str() {
                    "oneOf" => {
                        one_of_schema.one_of = loader::load_array_of_schemas(value)?;
                    }
                    _ => unimplemented!(),
                }
            }
        }
        Ok(one_of_schema)
    }
}

impl FromAnnotatedMapping<OneOfSchema> for OneOfSchema {
    fn from_annotated_mapping(
        mapping: &AnnotatedMapping<MarkedYaml>,
    ) -> crate::Result<OneOfSchema> {
        match mapping.get(&MarkedYaml::value_from_str("oneOf")) {
            Some(value) => {
                let one_of = loader::load_array_of_schemas_marked(value)?;
                Ok(OneOfSchema { one_of })
            }
            None => Err(generic_error!("No `oneOf` key found!")),
        }
    }
}
