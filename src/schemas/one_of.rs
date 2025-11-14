use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::YamlData;

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

    fn try_from(value: &MarkedYaml) -> Result<Self, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
            OneOfSchema::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for OneOfSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        match mapping.get(&MarkedYaml::value_from_str("oneOf")) {
            Some(value) => {
                let one_of = loader::load_array_of_schemas_marked(value)?;
                Ok(OneOfSchema { one_of })
            }
            None => Err(generic_error!("No `oneOf` key found!")),
        }
    }
}
