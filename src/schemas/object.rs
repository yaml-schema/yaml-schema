use std::collections::HashMap;

use crate::BoolOrTypedSchema;
use crate::YamlSchema;

/// An object schema
#[derive(Debug, Default, PartialEq)]
pub struct ObjectSchema {
    pub metadata: Option<HashMap<String, String>>,
    pub properties: Option<HashMap<String, YamlSchema>>,
    pub required: Option<Vec<String>>,
    pub additional_properties: Option<BoolOrTypedSchema>,
    pub pattern_properties: Option<HashMap<String, YamlSchema>>,
    pub property_names: Option<String>,
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
}

impl std::fmt::Display for ObjectSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Object {:?}", self)
    }
}
