// The loader module loads the YAML schema from a file into the in-memory model

use std::fs;
use std::rc::Rc;

use hashlink::LinkedHashMap;
use log::debug;
use saphyr::{AnnotatedMapping, LoadableYamlNode, MarkedYaml, Scalar, YamlData};

use crate::utils::{format_marker, saphyr_yaml_string, try_unwrap_saphyr_scalar};
use crate::AnyOfSchema;
use crate::BoolOrTypedSchema;
use crate::ConstSchema;
use crate::EnumSchema;
use crate::Error;
use crate::NotSchema;
use crate::Number;
use crate::OneOfSchema;
use crate::Reference;
use crate::Result;
use crate::RootSchema;
use crate::Schema;
use crate::TypedSchema;
use crate::YamlSchema;

pub fn load_file<S: Into<String>>(path: S) -> Result<RootSchema> {
    let path_s = path.into();
    let fs_metadata = fs::metadata(&path_s)?;
    if !fs_metadata.is_file() {
        return Err(Error::FileNotFound(path_s.clone()));
    }
    let s = fs::read_to_string(&path_s)?;
    let docs = MarkedYaml::load_from_str(&s)?;
    if docs.is_empty() {
        return Ok(RootSchema::new(YamlSchema::empty())); // empty schema
    }
    load_from_doc(docs.first().unwrap())
}

pub fn load_from_doc(doc: &MarkedYaml) -> Result<RootSchema> {
    let mut loader = RootLoader::new();
    match &doc.data {
        YamlData::Value(scalar) => match scalar {
            Scalar::Boolean(r#bool) => {
                loader.set_schema(YamlSchema::boolean_literal(*r#bool));
            }
            Scalar::Null => {
                loader.set_schema(YamlSchema::null());
            }
            Scalar::String(s) => match s.as_ref() {
                "true" => {
                    loader.set_schema(YamlSchema::boolean_literal(true));
                }
                "false" => {
                    loader.set_schema(YamlSchema::boolean_literal(false));
                }
                s => return Err(generic_error!("Expected true or false, but got: {}", s)),
            },
            _ => {
                return Err(generic_error!(
                    "Don't know how to a handle scalar: {:?}",
                    scalar
                ))
            }
        },
        _ => {
            if doc.data.is_mapping() {
                debug!("Found mapping: {doc:?}, trying to load as YamlSchema");
                loader.set_schema(doc.try_into()?);
            } else {
                return Err(generic_error!("Don't know how to load: {:?}", doc));
            }
        }
    }
    Ok(loader.into()) // See From<Loader> for RootSchema below
}

pub fn load_from_str(s: &str) -> Result<RootSchema> {
    let docs = MarkedYaml::load_from_str(s)?;
    let first_doc = docs.first().unwrap();
    Ok(load_from_doc(first_doc).unwrap())
}

#[derive(Debug, Default)]
struct RootLoader {
    pub id: Option<String>,
    pub meta_schema: Option<String>,
    pub title: Option<String>,
    pub defs: Option<LinkedHashMap<String, YamlSchema>>,
    pub description: Option<String>,
    pub schema: Option<YamlSchema>,
}

impl RootLoader {
    fn new() -> Self {
        RootLoader::default()
    }

    /// Set the loader schema
    /// Just a convenience function to avoid having to write
    /// `self.schema = Some(schema);`
    fn set_schema(&mut self, schema: YamlSchema) {
        self.schema = Some(schema);
    }

    fn load_root_schema(&mut self, mapping: &AnnotatedMapping<MarkedYaml>) -> Result<()> {
        // We can't remove the annotations, so we simply construct a new AnnotatedMapping containing
        // the 'data' nodes only
        let mut data = AnnotatedMapping::new();

        for (key, value) in mapping.iter() {
            match &key.data {
                YamlData::Value(scalar) => match scalar {
                    Scalar::String(s) => match s.as_ref() {
                        "$id" => {
                            self.id = Some(marked_yaml_to_string(value, "$id must be a string")?)
                        }
                        "$schema" => {
                            self.meta_schema =
                                Some(marked_yaml_to_string(value, "$schema must be a string")?)
                        }
                        "title" => {
                            self.title =
                                Some(marked_yaml_to_string(value, "title must be a string")?)
                        }
                        "description" => {
                            self.description = Some(marked_yaml_to_string(
                                value,
                                "description must be a string",
                            )?)
                        }
                        "$defs" | "definitions" => {
                            if let YamlData::Mapping(mapping) = &value.data {
                                let mut defs = LinkedHashMap::new();
                                for (key, value) in mapping.iter() {
                                    if let Ok(key_string) =
                                        marked_yaml_to_string(key, "key must be a string")
                                    {
                                        if let YamlData::Mapping(mapping) = &value.data {
                                            let schema: YamlSchema = mapping.try_into()?;
                                            defs.insert(key_string, schema);
                                        } else {
                                            return Err(generic_error!(
                                                "{} {} Expected a hash for {}, but got: {:#?}",
                                                format_marker(&value.span.start),
                                                s,
                                                key_string,
                                                value
                                            ));
                                        }
                                    } else {
                                        return Err(generic_error!(
                                            "{} Expected a string key, but got: {:#?}",
                                            s,
                                            value
                                        ));
                                    }
                                }
                                self.defs = Some(defs);
                            } else {
                                return Err(generic_error!(
                                    "{} Expected a hash, but got: {:#?}",
                                    s,
                                    value
                                ));
                            }
                        }
                        _ => {
                            data.insert(key.clone(), value.clone());
                        }
                    },
                    _ => {
                        data.insert(key.clone(), value.clone());
                    }
                },
                _ => {
                    return Err(expected_scalar!(
                        "{} Expected scalar key, but got: {:#?}",
                        format_marker(&key.span.start),
                        key
                    ))
                }
            }
        }
        let yaml_schema: YamlSchema = (&data).try_into()?;
        self.schema = Some(yaml_schema);
        Ok(())
    }
}

/// Convert a Loader to a RootSchema
/// Just sets the schema to a YamlSchema::Empty if the loader schema is None
impl From<RootLoader> for RootSchema {
    fn from(loader: RootLoader) -> Self {
        RootSchema {
            id: loader.id,
            meta_schema: loader.meta_schema,
            defs: loader.defs,
            schema: Rc::new(loader.schema.unwrap_or(YamlSchema::empty())),
        }
    }
}

/// "type" key
const TYPE: saphyr::Yaml = saphyr_yaml_string("type");
/// "enum" key
const ENUM: saphyr::Yaml = saphyr_yaml_string("enum");
/// "const" key
const CONST: saphyr::Yaml = saphyr_yaml_string("const");
/// "anyOf" key
const ANY_OF: saphyr::Yaml = saphyr_yaml_string("anyOf");
/// "oneOf" key
const ONE_OF: saphyr::Yaml = saphyr_yaml_string("oneOf");
/// "not" key
const NOT: saphyr::Yaml = saphyr_yaml_string("not");

impl FromSaphyrMapping<Option<Schema>> for Schema {
    fn from_mapping(mapping: &saphyr::Mapping) -> Result<Option<Schema>> {
        if mapping.is_empty() {
            Ok(None)
        } else if mapping.contains_key(&TYPE) {
            match TypedSchema::from_mapping(mapping) {
                Ok(typed_schema) => Ok(Some(typed_schema.into())),
                Err(e) => Err(e),
            }
        } else if mapping.contains_key(&ENUM) {
            let enum_schema = EnumSchema::from_mapping(mapping)?;
            return Ok(Some(Schema::Enum(enum_schema)));
        } else if mapping.contains_key(&CONST) {
            let const_schema = ConstSchema::from_mapping(mapping)?;
            return Ok(Some(Schema::Const(const_schema)));
        } else if mapping.contains_key(&ANY_OF) {
            let any_of_schema = AnyOfSchema::from_mapping(mapping)?;
            return Ok(Some(Schema::AnyOf(any_of_schema)));
        } else if mapping.contains_key(&ONE_OF) {
            let one_of_schema = OneOfSchema::from_mapping(mapping)?;
            return Ok(Some(Schema::OneOf(one_of_schema)));
        } else if mapping.contains_key(&NOT) {
            let not_schema = NotSchema::from_mapping(mapping)?;
            return Ok(Some(Schema::Not(not_schema)));
        } else {
            return Err(generic_error!(
                "(FromSaphyrMapping) Don't know how to construct schema: {:#?}",
                mapping
            ));
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for YamlSchema {
    type Error = Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> Result<Self> {
        let mut metadata: LinkedHashMap<String, String> = LinkedHashMap::new();
        let mut r#ref: Option<Reference> = None;
        let mut data = AnnotatedMapping::new();

        for (key, value) in mapping.iter() {
            match &key.data {
                YamlData::Value(Scalar::String(s)) => {
                    match s.as_ref() {
                        "$id" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "$id must be a string")?,
                            );
                        }
                        "$schema" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "$schema must be a string")?,
                            );
                        }
                        "$ref" => {
                            r#ref = Some(value.try_into()?);
                            // TODO: What?
                        }
                        "title" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "title must be a string")?,
                            );
                        }
                        "description" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "description must be a string")?,
                            );
                        }
                        _ => {
                            data.insert(key.clone(), value.clone());
                        }
                    }
                }
                _ => {
                    data.insert(key.clone(), value.clone());
                }
            }
        }
        let schema = Some(Schema::from_annotated_mapping(&data)?);
        Ok(YamlSchema {
            metadata: if metadata.is_empty() {
                None
            } else {
                Some(metadata)
            },
            schema,
            r#ref,
        })
    }
}

/// Try to convert a saphyr::Mapping into the desired (schema) type
pub trait FromSaphyrMapping<T> {
    fn from_mapping(mapping: &saphyr::Mapping) -> Result<T>;
}

pub trait FromAnnotatedMapping<T> {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> Result<T>;
}

pub fn load_string_value(value: &saphyr::Yaml) -> Result<String> {
    // When RustRover stops complaining about let chains (Rust 1.88), can rewrite the ff.
    if let saphyr::Yaml::Value(Scalar::String(s)) = value {
        return Ok(s.to_string());
    }

    Err(expected_scalar!(
        "Expected a string value, but got: {:?}",
        value
    ))
}

pub fn yaml_to_string<S: Into<String> + Copy>(yaml: &saphyr::Yaml, msg: S) -> Result<String> {
    load_string_value(yaml).map_err(|_| generic_error!("{}", msg.into()))
}

pub fn marked_yaml_to_string<S: Into<String> + Copy>(yaml: &MarkedYaml, msg: S) -> Result<String> {
    if let YamlData::Value(Scalar::String(s)) = &yaml.data {
        Ok(s.to_string())
    } else {
        Err(generic_error!("{}", msg.into()))
    }
}

pub fn load_array_of_schemas(value: &saphyr::Yaml) -> Result<Vec<YamlSchema>> {
    if let saphyr::Yaml::Sequence(values) = value {
        values
            .iter()
            .map(|v| match v {
                saphyr::Yaml::Mapping(mapping) => YamlSchema::from_mapping(mapping),
                _ => Err(generic_error!("Expected a mapping, but got: {:?}", v)),
            })
            .collect::<Result<Vec<YamlSchema>>>()
    } else {
        Err(generic_error!("Expected a sequence, but got: {:?}", value))
    }
}

pub fn load_array_of_schemas_marked(value: &MarkedYaml) -> Result<Vec<YamlSchema>> {
    if let YamlData::Sequence(values) = &value.data {
        values
            .iter()
            .map(|v| {
                if v.is_mapping() {
                    v.try_into()
                } else {
                    Err(generic_error!("Expected a mapping, but got: {:?}", v))
                }
            })
            .collect::<Result<Vec<YamlSchema>>>()
    } else {
        Err(generic_error!(
            "{} Expected a sequence, but got: {:?}",
            format_marker(&value.span.start),
            value
        ))
    }
}

pub fn load_integer(value: &saphyr::Yaml) -> Result<i64> {
    let scalar = try_unwrap_saphyr_scalar(value)?;
    match scalar {
        saphyr::Scalar::Integer(i) => Ok(*i),
        _ => Err(unsupported_type!(
            "Expected type: integer, but got: {:?}",
            value
        )),
    }
}

pub fn load_integer_marked(value: &MarkedYaml) -> Result<i64> {
    if let YamlData::Value(Scalar::Integer(i)) = &value.data {
        Ok(*i)
    } else {
        Err(generic_error!(
            "{} Expected integer value, got: {:?}",
            format_marker(&value.span.start),
            value
        ))
    }
}

pub fn load_number(value: &saphyr::Yaml) -> Result<Number> {
    let scalar = try_unwrap_saphyr_scalar(value)?;
    match scalar {
        Scalar::Integer(i) => Ok(Number::integer(*i)),
        Scalar::FloatingPoint(o) => Ok(Number::float(o.into_inner())),
        _ => Err(unsupported_type!(
            "Expected type: integer or float, but got: {:?}",
            value
        )),
    }
}

pub fn load_array_items(value: &saphyr::Yaml) -> Result<BoolOrTypedSchema> {
    match value {
        saphyr::Yaml::Value(scalar) => {
            if let saphyr::Scalar::Boolean(b) = scalar {
                Ok(BoolOrTypedSchema::Boolean(*b))
            } else {
                Err(generic_error!(
                    "array: boolean or mapping with type or $ref, but got: {:?}",
                    value
                ))
            }
        }
        saphyr::Yaml::Mapping(mapping) => {
            if mapping.contains_key(&saphyr_yaml_string("$ref")) {
                let reference = Reference::from_mapping(mapping);
                Ok(BoolOrTypedSchema::Reference(reference?))
            } else if mapping.contains_key(&saphyr_yaml_string("type")) {
                let typed_schema = TypedSchema::from_mapping(mapping)?;
                Ok(BoolOrTypedSchema::TypedSchema(Box::new(typed_schema)))
            } else {
                Err(generic_error!(
                    "array: boolean or mapping with type or $ref, but got: {:?}",
                    value
                ))
            }
        }
        _ => Err(generic_error!(
            "array: boolean or mapping with type or $ref, but got: {:?}",
            value
        )),
    }
}

pub fn load_array_items_marked(value: &MarkedYaml) -> Result<BoolOrTypedSchema> {
    match &value.data {
        YamlData::Value(scalar) => {
            if let Scalar::Boolean(b) = scalar {
                Ok(BoolOrTypedSchema::Boolean(*b))
            } else {
                Err(generic_error!(
                    "array: boolean or mapping with type or $ref, but got: {:?}",
                    value
                ))
            }
        }
        YamlData::Mapping(mapping) => {
            if mapping.contains_key(&MarkedYaml::value_from_str("$ref")) {
                let reference = value.try_into()?;
                Ok(BoolOrTypedSchema::Reference(reference))
            } else if mapping.contains_key(&MarkedYaml::value_from_str("type")) {
                let typed_schema = TypedSchema::from_annotated_mapping(mapping)?;
                Ok(BoolOrTypedSchema::TypedSchema(Box::new(typed_schema)))
            } else {
                Err(generic_error!(
                    "array: boolean or mapping with type or $ref, but got: {:?}",
                    value
                ))
            }
        }
        _ => Err(generic_error!(
            "array: boolean or mapping with type or $ref, but got: {:?}",
            value
        )),
    }
}

fn scalar_to_string(scalar: &saphyr::Scalar) -> String {
    match scalar {
        Scalar::String(s) => s.to_string(),
        Scalar::Integer(i) => i.to_string(),
        Scalar::FloatingPoint(f) => f.to_string(),
        Scalar::Boolean(b) => b.to_string(),
        Scalar::Null => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::ArraySchema;
    use crate::ConstValue;
    use crate::IntegerSchema;
    use crate::StringSchema;

    use super::*;

    #[test]
    fn test_boolean_literal_true() {
        let root_schema = load_from_doc(&MarkedYaml::value_from_str("true")).unwrap();
        assert_eq!(
            *root_schema.schema.as_ref(),
            YamlSchema::boolean_literal(true)
        );
    }

    #[test]
    fn test_boolean_literal_false() {
        let root_schema = load_from_doc(&MarkedYaml::value_from_str("false")).unwrap();
        assert_eq!(
            *root_schema.schema.as_ref(),
            YamlSchema::boolean_literal(false)
        );
    }

    #[test]
    fn test_const_string() {
        let docs = MarkedYaml::load_from_str("const: string value").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let const_schema = ConstSchema {
            r#const: ConstValue::string("string value"),
        };
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Const(const_schema)
        );
    }

    #[test]
    fn test_const_integer() {
        let docs = MarkedYaml::load_from_str("const: 42").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let const_schema = ConstSchema {
            r#const: ConstValue::integer(42),
        };
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Const(const_schema)
        );
    }

    #[test]
    fn test_type_foo_should_error() {
        let docs = MarkedYaml::load_from_str("type: foo").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap());
        assert!(root_schema.is_err());
        assert_eq!(
            root_schema.unwrap_err().to_string(),
            "Unsupported type 'foo'!"
        );
    }

    #[test]
    fn test_type_string() {
        let docs = MarkedYaml::load_from_str("type: string").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let string_schema = StringSchema::default();
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::String(string_schema)
        );
    }

    #[test]
    fn test_type_object_with_string_with_description() {
        let root_schema = load_from_str(
            r#"
            type: object
            properties:
                name:
                    type: string
                    description: This is a description
        "#,
        )
        .unwrap();
        let root_schema_schema = &root_schema.schema.as_ref().schema.as_ref().unwrap();
        let expected = StringSchema::default();
        if let Schema::Object(object_schema) = root_schema_schema {
            let name_property = object_schema
                .properties
                .as_ref()
                .expect("Expected properties")
                .get("name")
                .expect("Expected name property");
            let description = name_property
                .metadata
                .as_ref()
                .expect("Expected metadata")
                .get("description")
                .expect("Expected description");
            assert_eq!(description, "This is a description");
            if let Schema::String(actual) = &name_property.schema.as_ref().unwrap() {
                assert_eq!(&expected, actual);
            } else {
                panic!(
                    "Expected Schema::String, but got: {:?}",
                    name_property.schema
                );
            }
        } else {
            panic!("Expected Schema::Object, but got: {root_schema_schema:?}");
        }
    }

    #[test]
    fn test_type_string_with_pattern() {
        let root_schema = load_from_str(
            r#"
        type: string
        pattern: "^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$"
        "#,
        )
        .unwrap();
        let expected = StringSchema {
            pattern: Some(Regex::new("^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$").unwrap()),
            ..Default::default()
        };
        let root_schema_schema = root_schema.schema.as_ref().schema.as_ref().unwrap();
        assert_eq!(root_schema_schema, &Schema::String(expected));
    }

    #[test]
    fn test_array_constructor_items_true() {
        let mut mapping = saphyr::Mapping::new();
        mapping.insert(saphyr_yaml_string("type"), saphyr_yaml_string("array"));
        mapping.insert(
            saphyr_yaml_string("items"),
            saphyr::Yaml::Value(saphyr::Scalar::Boolean(true)),
        );
        let array_schema = ArraySchema::from_mapping(&mapping).unwrap();
        assert_eq!(
            array_schema,
            ArraySchema {
                items: Some(BoolOrTypedSchema::Boolean(true)),
                prefix_items: None,
                contains: None
            }
        );
    }

    #[test]
    fn test_integer_schema() {
        let docs = MarkedYaml::load_from_str("type: integer").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let integer_schema = IntegerSchema::default();
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Integer(integer_schema)
        );
    }

    #[test]
    fn test_enum() {
        let root_schema = load_from_str(
            r#"
        enum:
          - foo
          - bar
          - baz
        "#,
        )
        .unwrap();
        let enum_values = ["foo", "bar", "baz"]
            .iter()
            .map(|s| ConstValue::string(s.to_string()))
            .collect();
        let enum_schema = EnumSchema {
            r#enum: enum_values,
        };
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Enum(enum_schema)
        );
    }

    #[test]
    fn test_enum_without_type() {
        let root_schema = load_from_str(
            r#"
            enum:
              - red
              - amber
              - green
              - null
              - 42
            "#,
        )
        .unwrap();
        let enum_values = vec![
            ConstValue::string("red".to_string()),
            ConstValue::string("amber".to_string()),
            ConstValue::string("green".to_string()),
            ConstValue::null(),
            ConstValue::integer(42),
        ];
        let enum_schema = EnumSchema {
            r#enum: enum_values,
        };
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Enum(enum_schema)
        );
    }

    #[test]
    fn test_one_of_with_ref() {
        let root_schema = load_from_str(
            r##"
            $defs:
              foo:
                type: boolean
            oneOf:
              - type: string
              - $ref: "#/$defs/foo"
            "##,
        )
        .unwrap();
        println!("root_schema: {root_schema:#?}");
        let root_schema_schema = root_schema.schema.as_ref().schema.as_ref().unwrap();
        if let Schema::OneOf(one_of_schema) = root_schema_schema {
            println!("one_of_schema: {one_of_schema:#?}");
        } else {
            panic!("Expected Schema::OneOf, but got: {root_schema_schema:?}");
        }

        let s = r#"
        false
        "#;
        let docs = MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::with_root_schema(&root_schema, true);
        let result = root_schema.validate(&context, value);
        println!("result: {result:#?}");
        assert!(result.is_ok());
        for error in context.errors.borrow().iter() {
            println!("error: {error:#?}");
        }
        assert!(!context.has_errors());
    }
}
