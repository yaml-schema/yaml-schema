use std::collections::HashMap;

use hashlink::LinkedHashMap;
use log::debug;
use regex::Regex;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::AnyOfSchema;
use crate::BoolOrTypedSchema;
use crate::Error;
use crate::Reference;
use crate::Result;
use crate::StringSchema;
use crate::TypedSchema;
use crate::YamlSchema;
use crate::loader::load_array_of_schemas_marked;
use crate::loader::load_integer_marked;
use crate::schemas::BaseSchema;
use crate::schemas::SchemaMetadata;
use crate::utils::format_marker;
use crate::utils::hash_map;
use crate::utils::linked_hash_map;

/// An object schema
#[derive(Debug, Default, PartialEq)]
pub struct ObjectSchema {
    pub base: BaseSchema,
    pub metadata: Option<HashMap<String, String>>,
    pub properties: Option<LinkedHashMap<String, YamlSchema>>,
    pub required: Option<Vec<String>>,
    pub additional_properties: Option<BoolOrTypedSchema>,
    pub pattern_properties: Option<LinkedHashMap<String, YamlSchema>>,
    pub property_names: Option<StringSchema>,
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
    pub any_of: Option<AnyOfSchema>,
}

impl SchemaMetadata for ObjectSchema {
    fn get_accepted_keys() -> &'static [&'static str] {
        &[
            "properties",
            "required",
            "additionalProperties",
            "patternProperties",
            "propertyNames",
            "minProperties",
            "maxProperties",
            "anyOf",
        ]
    }
}

impl ObjectSchema {
    pub fn from_base(base: BaseSchema) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    pub fn builder() -> ObjectSchemaBuilder {
        ObjectSchemaBuilder::new()
    }
}

impl TryFrom<&MarkedYaml<'_>> for ObjectSchema {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml<'_>) -> Result<Self> {
        debug!("[ObjectSchema]: TryFrom {marked_yaml:?}");
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            Ok(ObjectSchema::try_from(mapping)?)
        } else {
            Err(expected_mapping!(marked_yaml))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for ObjectSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let mut object_schema = ObjectSchema::from_base(BaseSchema::try_from(mapping)?);
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(s)) = &key.data {
                if object_schema.base.handle_key_value(s, value)?.is_none() {
                    match s.as_ref() {
                        "properties" => {
                            let properties = load_properties_marked(value)?;
                            object_schema.properties = Some(properties);
                        }
                        "additionalProperties" => {
                            let additional_properties = load_additional_properties_marked(value)?;
                            object_schema.additional_properties = Some(additional_properties);
                        }
                        "minProperties" => {
                            object_schema.min_properties =
                                Some(load_integer_marked(value)? as usize);
                        }
                        "maxProperties" => {
                            object_schema.max_properties =
                                Some(load_integer_marked(value)? as usize);
                        }
                        "patternProperties" => {
                            let pattern_properties = load_properties_marked(value)?;
                            object_schema.pattern_properties = Some(pattern_properties);
                        }
                        "propertyNames" => {
                            if let YamlData::Mapping(mapping) = &value.data {
                                let pattern_key = MarkedYaml::value_from_str("pattern");
                                if !mapping.contains_key(&pattern_key) {
                                    return Err(generic_error!(
                                        "{} propertyNames: Missing required key: pattern",
                                        format_marker(&value.span.start)
                                    ));
                                }
                                if let YamlData::Value(Scalar::String(pattern)) =
                                    &mapping.get(&pattern_key).unwrap().data
                                {
                                    let regex = Regex::new(pattern.as_ref()).map_err(|_e| {
                                        Error::InvalidRegularExpression(pattern.to_string())
                                    })?;
                                    object_schema.property_names =
                                        Some(StringSchema::builder().pattern(regex).build());
                                }
                            } else {
                                return Err(unsupported_type!(
                                    "propertyNames: Expected a mapping, but got: {:?}",
                                    value
                                ));
                            }
                        }
                        "anyOf" => {
                            let any_of = load_array_of_schemas_marked(value)?;
                            let any_of_schema = AnyOfSchema { any_of };
                            object_schema.any_of = Some(any_of_schema);
                        }
                        "required" => {
                            if let YamlData::Sequence(values) = &value.data {
                                let required = values
                                    .iter()
                                    .map(|v| {
                                        if let YamlData::Value(Scalar::String(s)) = &v.data {
                                            Ok(s.to_string())
                                        } else {
                                            Err(generic_error!(
                                                "{} Expected a string, got {:?}",
                                                format_marker(&v.span.start),
                                                v
                                            ))
                                        }
                                    })
                                    .collect::<Result<Vec<String>>>()?;
                                object_schema.required = Some(required);
                            } else {
                                return Err(unsupported_type!(
                                    "required: Expected an array, but got: {:?}",
                                    value
                                ));
                            }
                        }
                        // Maybe this should be handled by the base schema?
                        "type" => {
                            if let YamlData::Value(Scalar::String(s)) = &value.data {
                                if s != "object" {
                                    return Err(unsupported_type!(
                                        "Expected type: object, but got: {}",
                                        s
                                    ));
                                }
                            } else {
                                return Err(expected_type_is_string!(value));
                            }
                        }
                        _ => {
                            if s.starts_with("$") {
                                if let YamlData::Value(Scalar::String(value)) = &key.data {
                                    if object_schema.metadata.is_none() {
                                        object_schema.metadata = Some(HashMap::new());
                                    }
                                    object_schema
                                        .metadata
                                        .as_mut()
                                        .unwrap()
                                        .insert(s.to_string(), value.to_string());
                                } else {
                                    return Err(generic_error!(
                                        "{} Expected a string value but got {:?}",
                                        format_marker(&value.span.start),
                                        value.data
                                    ));
                                }
                            } else {
                                unimplemented!("Unsupported key for type: object: {}", s);
                            }
                        }
                    }
                }
            } else {
                return Err(generic_error!(
                    "{} Expected a scalar key, got: {:#?}",
                    format_marker(&key.span.start),
                    key
                ));
            }
        }
        Ok(object_schema)
    }
}

fn load_properties_marked(value: &MarkedYaml) -> Result<LinkedHashMap<String, YamlSchema>> {
    if let YamlData::Mapping(mapping) = &value.data {
        let mut properties = LinkedHashMap::new();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(key)) = &key.data {
                if key.as_ref() == "$ref" {
                    let reference: Reference = value.try_into()?;
                    properties.insert(key.to_string(), YamlSchema::reference(reference));
                } else if value.data.is_mapping() {
                    let schema: YamlSchema = value.try_into()?;
                    properties.insert(key.to_string(), schema);
                } else {
                    return Err(generic_error!(
                        "properties: Expected a mapping for \"{}\", but got: {:?}",
                        key,
                        value
                    ));
                }
            } else {
                return Err(generic_error!(
                    "{} Expected a string key, but got: {:?}",
                    format_marker(&key.span.start),
                    key
                ));
            }
        }
        Ok(properties)
    } else {
        Err(generic_error!(
            "{} properties: expected a mapping, but got: {:#?}",
            format_marker(&value.span.start),
            value
        ))
    }
}

fn load_additional_properties_marked(marked_yaml: &MarkedYaml) -> Result<BoolOrTypedSchema> {
    match &marked_yaml.data {
        YamlData::Value(scalar) => match scalar {
            Scalar::Boolean(b) => Ok(BoolOrTypedSchema::Boolean(*b)),
            _ => Err(generic_error!(
                "{} Expected a boolean scalar, but got: {:#?}",
                format_marker(&marked_yaml.span.start),
                scalar
            )),
        },
        YamlData::Mapping(mapping) => {
            let ref_key = MarkedYaml::value_from_str("$ref");
            if mapping.contains_key(&ref_key) {
                Ok(BoolOrTypedSchema::Reference(marked_yaml.try_into()?))
            } else {
                let schema: TypedSchema = marked_yaml.try_into()?;
                Ok(BoolOrTypedSchema::TypedSchema(Box::new(schema)))
            }
        }
        _ => Err(unsupported_type!(
            "Expected type: boolean or mapping, but got: {:?}",
            marked_yaml
        )),
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

    pub fn properties(&mut self, properties: LinkedHashMap<String, YamlSchema>) -> &mut Self {
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
            self.properties(linked_hash_map(key.into(), value))
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
        pattern_properties: LinkedHashMap<String, YamlSchema>,
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
            self.pattern_properties(linked_hash_map(key.into(), value))
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
    use crate::Validator;
    use saphyr::LoadableYamlNode;

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
            .property("type", YamlSchema::ref_str("schema_type"))
            .build();
        assert!(schema.properties.is_some());
        assert_eq!(
            *schema.properties.unwrap().get("type").unwrap(),
            YamlSchema::ref_str("schema_type")
        );
    }

    #[test]
    fn test_additional_properties_as_schema() {
        let docs = MarkedYaml::load_from_str(
            "
      type: object
      properties:
        number:
          type: number
        street_name:
          type: string
        street_type:
          enum: [Street, Avenue, Boulevard]
      additionalProperties:
        type: string",
        )
        .unwrap();

        let doc = docs.first().unwrap();

        let schema: ObjectSchema = doc.try_into().unwrap();

        let yaml_docs = MarkedYaml::load_from_str(
            "
number: 1600
street_name: Pennsylvania
street_type: Avenue
office_number: 201",
        )
        .unwrap();

        let yaml = yaml_docs.first().unwrap();

        let context = crate::Context::default();
        let result = schema.validate(&context, yaml);
        if result.is_err() {
            println!("{:?}", result.as_ref().unwrap());
            panic!("Validation failed: {:?}", result.as_ref().unwrap());
        }
        assert!(context.has_errors());
        for error in context.errors.as_ref().borrow().iter() {
            println!("{error:?}");
        }
    }

    #[test]
    fn test_object_schema_with_description() {
        let yaml = r#"
        type: object
        description: The description
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let marked_yaml = doc.first().unwrap();
        let object_schema = ObjectSchema::try_from(marked_yaml).unwrap();
        assert_eq!(
            object_schema.base.description,
            Some("The description".to_string())
        );
    }
}
