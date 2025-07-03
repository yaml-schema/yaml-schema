/// The `anyOf` schema is a schema that matches if any of the schemas in the `anyOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
use crate::utils::format_vec;
use crate::YamlSchema;

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
