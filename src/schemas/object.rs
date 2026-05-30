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
use crate::loader::marked_yaml_mapping_key_to_string;
use crate::schemas::BooleanOrSchema;
use crate::schemas::SchemaType;
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
    /// JSON Schema `propertyNames`: subschema validated against each mapping key.
    pub property_names: Option<YamlSchema>,
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
                        if value.data.is_mapping() {
                            let schema: YamlSchema = value.try_into()?;
                            validate_property_names_subschema(&schema, &value.span.start)?;
                            object_schema.property_names = Some(schema);
                        } else {
                            return Err(unsupported_type!(
                                "propertyNames: Expected a mapping (subschema), but got: {:?}",
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
                    "unevaluatedProperties" => {
                        // Loaded on `Subschema`; ignore here when parsing `type: object` mapping.
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
            let key_string = marked_yaml_mapping_key_to_string(key)?;
            if value.data.is_mapping() {
                let schema: YamlSchema = value.try_into()?;
                properties.insert(key_string, schema);
            } else {
                return Err(generic_error!(
                    "properties: Expected a mapping for \"{}\", but got: {:?}",
                    key_string,
                    value
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
            let pattern = marked_yaml_mapping_key_to_string(key)?;
            let regex = Regex::new(pattern.as_ref())
                .map_err(|_e| Error::InvalidRegularExpression(pattern.clone()))?;
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
            let trigger = marked_yaml_mapping_key_to_string(key)?;
            let YamlData::Sequence(values) = &val.data else {
                return Err(unsupported_type!(
                    "{} dependentRequired: Expected array for key {:?}, got: {:?}",
                    format_marker(&val.span.start),
                    trigger,
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
                        trigger
                    ));
                }
                deps.push(dep);
            }
            out.insert(trigger, deps);
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
            let name = marked_yaml_mapping_key_to_string(key)?;
            if !val.data.is_mapping() {
                return Err(generic_error!(
                    "dependentSchemas: Expected a mapping for {:?}, but got: {:?}",
                    name,
                    val.data
                ));
            }
            let schema: YamlSchema = val.try_into()?;
            out.insert(name, schema);
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

const PROPERTY_NAMES_SCALAR_TYPES: &[&str] = &["string", "integer", "number", "boolean", "null"];

/// Reject `propertyNames` subschemas that target array or object instances (mapping keys are scalars).
fn validate_property_names_subschema(schema: &YamlSchema, location: &saphyr::Marker) -> Result<()> {
    match schema {
        YamlSchema::Empty | YamlSchema::Null | YamlSchema::BooleanLiteral(_) => Ok(()),
        YamlSchema::Subschema(subschema) => {
            if subschema.r#type.is_or_contains("array") || subschema.r#type.is_or_contains("object")
            {
                return Err(property_names_complex_type_error(
                    location,
                    &subschema.r#type,
                ));
            }
            if subschema.array_schema.is_some() || subschema.object_schema.is_some() {
                return Err(generic_error!(
                    "{} propertyNames: array and object schemas are not allowed; only scalar types ({}) are permitted",
                    format_marker(location),
                    PROPERTY_NAMES_SCALAR_TYPES.join(", ")
                ));
            }
            if let Some(one_of) = &subschema.one_of {
                for branch in &one_of.one_of {
                    validate_property_names_subschema(branch, location)?;
                }
            }
            if let Some(any_of) = &subschema.any_of {
                for branch in &any_of.any_of {
                    validate_property_names_subschema(branch, location)?;
                }
            }
            if let Some(all_of) = &subschema.all_of {
                for branch in &all_of.all_of {
                    validate_property_names_subschema(branch, location)?;
                }
            }
            if let Some(not) = &subschema.not {
                validate_property_names_subschema(&not.not, location)?;
            }
            if let Some(conditional) = &subschema.if_then_else {
                validate_property_names_subschema(&conditional.if_schema, location)?;
                if let Some(then_schema) = &conditional.then_schema {
                    validate_property_names_subschema(then_schema, location)?;
                }
                if let Some(else_schema) = &conditional.else_schema {
                    validate_property_names_subschema(else_schema, location)?;
                }
            }
            Ok(())
        }
    }
}

fn property_names_complex_type_error(location: &saphyr::Marker, schema_type: &SchemaType) -> Error {
    let type_name = match schema_type {
        SchemaType::None => "none".to_string(),
        SchemaType::Single(s) => s.clone(),
        SchemaType::Multiple(values) => values.join(", "),
    };
    generic_error!(
        "{} propertyNames: type '{}' is not allowed; only scalar types ({}) are permitted",
        format_marker(location),
        type_name,
        PROPERTY_NAMES_SCALAR_TYPES.join(", ")
    )
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

    pub fn property_names(&mut self, schema: YamlSchema) -> &mut Self {
        self.0.property_names = Some(schema);
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
    fn test_properties_numeric_mapping_key_loads() {
        let yaml = "
        type: object
        properties:
          1:
            type: string";
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let os: ObjectSchema = doc.first().unwrap().try_into().unwrap();
        assert!(
            os.properties.as_ref().unwrap().contains_key("1"),
            "unquoted numeric mapping key should become string property name \"1\""
        );
    }

    #[test]
    fn test_property_names_loads_integer_schema() {
        let yaml = r#"
        type: object
        propertyNames:
          type: integer
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let os: ObjectSchema = doc.first().unwrap().try_into().unwrap();
        assert!(
            os.property_names.is_some(),
            "propertyNames subschema should be loaded"
        );
    }

    #[test]
    fn test_property_names_rejects_non_mapping() {
        let yaml = r#"
        type: object
        propertyNames: integer
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        assert!(ObjectSchema::try_from(doc.first().unwrap()).is_err());
    }

    #[test]
    fn test_property_names_validation_accepts_integer_keys() {
        let yaml = r#"
        type: object
        propertyNames:
          type: integer
        "#;
        let schema: ObjectSchema = MarkedYaml::load_from_str(yaml)
            .unwrap()
            .first()
            .unwrap()
            .try_into()
            .unwrap();
        let inst = MarkedYaml::load_from_str("1: a\n2: b").unwrap();
        let ctx = crate::Context::default();
        schema.validate(&ctx, inst.first().unwrap()).unwrap();
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_property_names_validation_rejects_string_key() {
        let yaml = r#"
        type: object
        propertyNames:
          type: integer
        "#;
        let schema: ObjectSchema = MarkedYaml::load_from_str(yaml)
            .unwrap()
            .first()
            .unwrap()
            .try_into()
            .unwrap();
        let inst = MarkedYaml::load_from_str("x: 1").unwrap();
        let ctx = crate::Context::default();
        schema.validate(&ctx, inst.first().unwrap()).unwrap();
        assert!(ctx.has_errors(), "non-integer keys should surface errors");
    }

    #[test]
    fn test_property_names_rejects_type_array() {
        let yaml = r#"
        type: object
        propertyNames:
          type: array
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let err = ObjectSchema::try_from(doc.first().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("array and object schemas are not allowed")
                || err.to_string().contains("type"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_property_names_rejects_type_object() {
        let yaml = r#"
        type: object
        propertyNames:
          type: object
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let err = ObjectSchema::try_from(doc.first().unwrap()).unwrap_err();
        assert!(err.to_string().contains("not allowed"), "unexpected: {err}");
    }

    #[test]
    fn test_property_names_rejects_one_of_with_array_branch() {
        let yaml = r#"
        type: object
        propertyNames:
          oneOf:
            - type: string
            - type: array
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let err = ObjectSchema::try_from(doc.first().unwrap()).unwrap_err();
        assert!(err.to_string().contains("not allowed"), "unexpected: {err}");
    }

    #[test]
    fn test_property_names_implicit_string_type_accepts_pattern() {
        let yaml = r#"
        type: object
        propertyNames:
          pattern: "^[a-z]+$"
        "#;
        let schema: ObjectSchema = MarkedYaml::load_from_str(yaml)
            .unwrap()
            .first()
            .unwrap()
            .try_into()
            .unwrap();
        let ok = MarkedYaml::load_from_str("alpha: 1").unwrap();
        let ctx = crate::Context::default();
        schema.validate(&ctx, ok.first().unwrap()).unwrap();
        assert!(!ctx.has_errors());

        let bad = MarkedYaml::load_from_str("Beta: 1").unwrap();
        let ctx = crate::Context::default();
        schema.validate(&ctx, bad.first().unwrap()).unwrap();
        assert!(ctx.has_errors());
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
