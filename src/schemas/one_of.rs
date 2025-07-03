/// The `oneOf` schema is a schema that matches if one, and only one of the schemas in the `oneOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
use crate::utils::format_vec;
use crate::YamlSchema;

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
