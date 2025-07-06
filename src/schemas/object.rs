use std::collections::HashMap;

use crate::schemas::TypedSchema;
use crate::utils::hash_map;
use crate::BoolOrTypedSchema;
use crate::YamlSchema;
use crate::{AnyOfSchema, StringSchema};

/// An object schema
#[derive(Debug, Default, PartialEq)]
pub struct ObjectSchema {
    pub metadata: Option<HashMap<String, String>>,
    pub properties: Option<HashMap<String, YamlSchema>>,
    pub required: Option<Vec<String>>,
    pub additional_properties: Option<BoolOrTypedSchema>,
    pub pattern_properties: Option<HashMap<String, YamlSchema>>,
    pub property_names: Option<StringSchema>,
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
    pub any_of: Option<AnyOfSchema>,
}

impl ObjectSchema {
    pub fn builder() -> ObjectSchemaBuilder {
        ObjectSchemaBuilder::new()
    }
}

impl std::fmt::Display for ObjectSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Object {self:?}")
    }
}

pub struct ObjectSchemaBuilder(ObjectSchema);

impl Default for ObjectSchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectSchemaBuilder {
    pub fn new() -> Self {
        Self(ObjectSchema::default())
    }

    pub fn build(&mut self) -> ObjectSchema {
        std::mem::take(&mut self.0)
    }

    pub fn boxed(&mut self) -> Box<ObjectSchema> {
        Box::new(self.build())
    }

    pub fn metadata<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        if let Some(metadata) = self.0.metadata.as_mut() {
            metadata.insert(key.into(), value.into());
        } else {
            self.0.metadata = Some(hash_map(key.into(), value.into()));
        }
        self
    }

    pub fn properties(&mut self, properties: HashMap<String, YamlSchema>) -> &mut Self {
        self.0.properties = Some(properties);
        self
    }

    pub fn property<K>(&mut self, key: K, value: YamlSchema) -> &mut Self
    where
        K: Into<String>,
    {
        if let Some(properties) = self.0.properties.as_mut() {
            properties.insert(key.into(), value);
            self
        } else {
            self.properties(hash_map(key.into(), value))
        }
    }

    pub fn require<S>(&mut self, property_name: S) -> &mut Self
    where
        S: Into<String>,
    {
        if let Some(required) = self.0.required.as_mut() {
            required.push(property_name.into());
        } else {
            self.0.required = Some(vec![property_name.into()]);
        }
        self
    }

    pub fn additional_properties(&mut self, additional_properties: bool) -> &mut Self {
        self.0.additional_properties = Some(BoolOrTypedSchema::Boolean(additional_properties));
        self
    }

    pub fn additional_property_types(&mut self, typed_schema: TypedSchema) -> &mut Self {
        self.0.additional_properties = Some(BoolOrTypedSchema::TypedSchema(Box::new(typed_schema)));
        self
    }

    pub fn pattern_properties(
        &mut self,
        pattern_properties: HashMap<String, YamlSchema>,
    ) -> &mut Self {
        self.0.pattern_properties = Some(pattern_properties);
        self
    }

    pub fn pattern_property<K>(&mut self, key: K, value: YamlSchema) -> &mut Self
    where
        K: Into<String>,
    {
        if let Some(pattern_properties) = self.0.pattern_properties.as_mut() {
            pattern_properties.insert(key.into(), value);
            self
        } else {
            self.pattern_properties(hash_map(key.into(), value))
        }
    }

    pub fn property_names(&mut self, property_names: StringSchema) -> &mut Self {
        self.0.property_names = Some(property_names);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Reference;

    #[test]
    fn test_builder_default() {
        let schema = ObjectSchema::builder().build();
        assert_eq!(ObjectSchema::default(), schema);
    }

    #[test]
    fn test_builder_metadata() {
        let schema = ObjectSchema::builder()
            .metadata("description", "The description")
            .build();
        assert!(schema.metadata.is_some());
        assert_eq!(
            schema.metadata.unwrap().get("description").unwrap(),
            "The description"
        );
    }

    #[test]
    fn test_builder_properties() {
        let schema = ObjectSchema::builder()
            .property("type", YamlSchema::reference(Reference::new("schema_type")))
            .build();
        assert!(schema.properties.is_some());
        assert_eq!(
            *schema.properties.unwrap().get("type").unwrap(),
            YamlSchema::reference(Reference::new("schema_type"))
        );
    }
}
