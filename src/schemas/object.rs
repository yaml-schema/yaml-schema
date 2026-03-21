use std::collections::HashSet;
use std::fmt::Display;

use hashlink::LinkedHashMap;
use log::debug;
use regex::Regex;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::Error;
use crate::Result;
use crate::YamlSchema;
use crate::loader::load_integer_marked;
use crate::schemas::BooleanOrSchema;
use crate::schemas::StringSchema;
use crate::utils::format_annotated_mapping;
use crate::utils::format_marker;
use crate::utils::linked_hash_map;

/// A pattern property entry: a pre-compiled regex paired with its schema.
#[derive(Debug)]
pub struct PatternProperty {
    pub regex: Regex,
    pub schema: YamlSchema,
}

impl PartialEq for PatternProperty {
    fn eq(&self, other: &Self) -> bool {
        self.regex.as_str() == other.regex.as_str() && self.schema == other.schema
    }
}

/// An object schema
#[derive(Debug, Default, PartialEq)]
pub struct ObjectSchema {
    pub properties: Option<LinkedHashMap<String, YamlSchema>>,
    pub required: Option<Vec<String>>,
    pub additional_properties: Option<BooleanOrSchema>,
    pub pattern_properties: Option<Vec<PatternProperty>>,
    pub property_names: Option<StringSchema>,
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
    /// JSON Schema `dependentRequired`: when a trigger property is present, all listed properties must be present.
    pub dependent_required: Option<LinkedHashMap<String, Vec<String>>>,
    /// JSON Schema `dependentSchemas`: when a trigger property is present, the whole object must match the subschema.
    pub dependent_schemas: Option<LinkedHashMap<String, YamlSchema>>,
}

impl ObjectSchema {
    pub fn builder() -> ObjectSchemaBuilder {
        ObjectSchemaBuilder::new()
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for ObjectSchema {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml<'r>) -> Result<Self> {
        debug!("[ObjectSchema]: TryFrom {marked_yaml:?}");
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            Ok(ObjectSchema::try_from(mapping)?)
        } else {
            Err(expected_mapping!(marked_yaml))
        }
    }
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for ObjectSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'r, MarkedYaml<'r>>) -> crate::Result<Self> {
        debug!(
            "[ObjectSchema#try_from] Mapping: {}",
            format_annotated_mapping(mapping)
        );
        let mut object_schema = ObjectSchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(s)) = &key.data {
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
                        object_schema.min_properties = Some(load_integer_marked(value)? as usize);
                    }
                    "maxProperties" => {
                        object_schema.max_properties = Some(load_integer_marked(value)? as usize);
                    }
                    "patternProperties" => {
                        object_schema.pattern_properties =
                            Some(load_pattern_properties_marked(value)?);
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
                            if let Some(v) = &mapping.get(&pattern_key)
                                && let YamlData::Value(Scalar::String(pattern)) = &v.data
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
                    "dependentRequired" => {
                        object_schema.dependent_required =
                            Some(load_dependent_required_marked(value)?);
                    }
                    "dependentSchemas" => {
                        object_schema.dependent_schemas =
                            Some(load_dependent_schemas_marked(value)?);
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
                        debug!("Unsupported key for type: object: {}", s);
                    }
                }
            } else {
                return Err(expected_scalar!(
                    "{} Expected a scalar key, got: {:?}",
                    format_marker(&key.span.start),
                    key
                ));
            }
        }
        Ok(object_schema)
    }
}

fn load_properties_marked<'r>(value: &MarkedYaml<'r>) -> Result<LinkedHashMap<String, YamlSchema>> {
    if let YamlData::Mapping(mapping) = &value.data {
        let mut properties = LinkedHashMap::new();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(key)) = &key.data {
                if value.data.is_mapping() {
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
            "{} properties: expected a mapping, but got: {:?}",
            format_marker(&value.span.start),
            value
        ))
    }
}

fn load_pattern_properties_marked<'r>(value: &MarkedYaml<'r>) -> Result<Vec<PatternProperty>> {
    if let YamlData::Mapping(mapping) = &value.data {
        let mut pattern_properties = Vec::new();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(pattern)) = &key.data {
                let regex = Regex::new(pattern.as_ref())
                    .map_err(|_e| Error::InvalidRegularExpression(pattern.to_string()))?;
                if value.data.is_mapping() {
                    let schema: YamlSchema = value.try_into()?;
                    pattern_properties.push(PatternProperty { regex, schema });
                } else {
                    return Err(generic_error!(
                        "patternProperties: Expected a mapping for \"{}\", but got: {:?}",
                        pattern,
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
        Ok(pattern_properties)
    } else {
        Err(generic_error!(
            "{} patternProperties: expected a mapping, but got: {:?}",
            format_marker(&value.span.start),
            value
        ))
    }
}

fn load_dependent_required_marked<'r>(
    value: &MarkedYaml<'r>,
) -> Result<LinkedHashMap<String, Vec<String>>> {
    if let YamlData::Mapping(mapping) = &value.data {
        let mut out = LinkedHashMap::new();
        for (key, val) in mapping.iter() {
            let YamlData::Value(Scalar::String(trigger)) = &key.data else {
                return Err(generic_error!(
                    "{} dependentRequired: Expected string key, got: {:?}",
                    format_marker(&key.span.start),
                    key.data
                ));
            };
            let YamlData::Sequence(values) = &val.data else {
                return Err(unsupported_type!(
                    "{} dependentRequired: Expected array for key {:?}, got: {:?}",
                    format_marker(&val.span.start),
                    trigger.as_ref(),
                    val.data
                ));
            };
            let mut deps = Vec::new();
            let mut seen = HashSet::new();
            for v in values {
                let YamlData::Value(Scalar::String(s)) = &v.data else {
                    return Err(generic_error!(
                        "{} dependentRequired: Expected string in array, got: {:?}",
                        format_marker(&v.span.start),
                        v.data
                    ));
                };
                let dep = s.to_string();
                if !seen.insert(dep.clone()) {
                    return Err(generic_error!(
                        "{} dependentRequired: duplicate property name {:?} for trigger {:?}",
                        format_marker(&v.span.start),
                        dep,
                        trigger.as_ref()
                    ));
                }
                deps.push(dep);
            }
            out.insert(trigger.to_string(), deps);
        }
        Ok(out)
    } else {
        Err(generic_error!(
            "{} dependentRequired: expected a mapping, but got: {:?}",
            format_marker(&value.span.start),
            value.data
        ))
    }
}

fn load_dependent_schemas_marked<'r>(
    value: &MarkedYaml<'r>,
) -> Result<LinkedHashMap<String, YamlSchema>> {
    if let YamlData::Mapping(mapping) = &value.data {
        let mut out = LinkedHashMap::new();
        for (key, val) in mapping.iter() {
            let YamlData::Value(Scalar::String(name)) = &key.data else {
                return Err(generic_error!(
                    "{} dependentSchemas: Expected string key, got: {:?}",
                    format_marker(&key.span.start),
                    key.data
                ));
            };
            if !val.data.is_mapping() {
                return Err(generic_error!(
                    "dependentSchemas: Expected a mapping for {:?}, but got: {:?}",
                    name.as_ref(),
                    val.data
                ));
            }
            let schema: YamlSchema = val.try_into()?;
            out.insert(name.to_string(), schema);
        }
        Ok(out)
    } else {
        Err(generic_error!(
            "{} dependentSchemas: expected a mapping, but got: {:?}",
            format_marker(&value.span.start),
            value.data
        ))
    }
}

fn load_additional_properties_marked<'r>(marked_yaml: &MarkedYaml<'r>) -> Result<BooleanOrSchema> {
    match &marked_yaml.data {
        YamlData::Value(scalar) => match scalar {
            Scalar::Boolean(b) => Ok(BooleanOrSchema::Boolean(*b)),
            _ => Err(generic_error!(
                "{} Expected a boolean scalar, but got: {:?}",
                format_marker(&marked_yaml.span.start),
                scalar
            )),
        },
        YamlData::Mapping(_mapping) => marked_yaml.try_into().map(BooleanOrSchema::schema),
        _ => Err(unsupported_type!(
            "Expected type: boolean or mapping, but got: {:?}",
            marked_yaml
        )),
    }
}

impl Display for ObjectSchema {
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
        self.0.additional_properties = Some(BooleanOrSchema::Boolean(additional_properties));
        self
    }

    pub fn additional_property_types(&mut self, typed_schema: YamlSchema) -> &mut Self {
        self.0.additional_properties = Some(BooleanOrSchema::schema(typed_schema));
        self
    }

    pub fn pattern_properties(&mut self, pattern_properties: Vec<PatternProperty>) -> &mut Self {
        self.0.pattern_properties = Some(pattern_properties);
        self
    }

    /// Add a pattern property, compiling the regex pattern at build time.
    ///
    /// # Panics
    /// Panics if `pattern` is not a valid regex.
    pub fn pattern_property<K>(&mut self, pattern: K, schema: YamlSchema) -> &mut Self
    where
        K: AsRef<str>,
    {
        let regex = Regex::new(pattern.as_ref())
            .unwrap_or_else(|e| panic!("Invalid regex pattern '{}': {e}", pattern.as_ref()));
        let entry = PatternProperty { regex, schema };
        if let Some(pattern_properties) = self.0.pattern_properties.as_mut() {
            pattern_properties.push(entry);
        } else {
            self.0.pattern_properties = Some(vec![entry]);
        }
        self
    }

    pub fn property_names(&mut self, property_names: StringSchema) -> &mut Self {
        self.0.property_names = Some(property_names);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Validator, loader};
    use saphyr::LoadableYamlNode;

    #[test]
    fn test_builder_default() {
        let schema = ObjectSchema::builder().build();
        assert_eq!(ObjectSchema::default(), schema);
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
        assert!(result.is_ok(), "Validation failed: {result:?}");

        assert!(context.has_errors());
    }

    #[test]
    fn test_object_schema_with_description() {
        let yaml = r#"
        type: object
        description: The description
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let marked_yaml = doc.first().unwrap();
        let yaml_schema = YamlSchema::try_from(marked_yaml).unwrap();
        let YamlSchema::Subschema(object_schema) = &yaml_schema else {
            panic!("Expected Subschema, but got: {:?}", &yaml_schema);
        };
        assert_eq!(
            object_schema.metadata_and_annotations.description,
            Some("The description".to_string())
        );
    }

    #[test]
    fn test_object_schema_with_const_property() {
        let yaml = r#"
        type: object
        properties:
          const:
            type:
              - string
              - integer
              - number
              - boolean
        "#;
        let root_schema = loader::load_from_str(yaml).unwrap();
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        let Some(object_schema) = &subschema.object_schema else {
            panic!(
                "Expected ObjectSchema, but got: {:?}",
                &subschema.object_schema
            );
        };
        // Verify properties were loaded correctly
        assert!(
            object_schema
                .properties
                .as_ref()
                .unwrap()
                .contains_key("const")
        );
    }

    #[test]
    fn test_dependent_required_loads() {
        let yaml = r#"
        type: object
        dependentRequired:
          a:
            - b
            - c
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let os: ObjectSchema = doc.first().unwrap().try_into().unwrap();
        let dr = os.dependent_required.as_ref().unwrap();
        assert_eq!(dr.get("a"), Some(&vec!["b".to_string(), "c".to_string()]));
    }

    #[test]
    fn test_dependent_required_rejects_duplicate_dep() {
        let yaml = r#"
        type: object
        dependentRequired:
          a:
            - b
            - b
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let err = ObjectSchema::try_from(doc.first().unwrap()).unwrap_err();
        assert!(
            err.to_string().contains("duplicate property name"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_dependent_schemas_loads() {
        let yaml = r#"
        type: object
        dependentSchemas:
          foo:
            type: object
            required:
              - bar
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let os: ObjectSchema = doc.first().unwrap().try_into().unwrap();
        assert!(os.dependent_schemas.is_some());
        let ds = os.dependent_schemas.as_ref().unwrap();
        assert!(ds.contains_key("foo"));
    }
}
