use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::ArraySchema;
use crate::IntegerSchema;
use crate::NumberSchema;
use crate::ObjectSchema;
use crate::Result;
use crate::StringSchema;
use crate::Validator;
use crate::loader::FromAnnotatedMapping;
use crate::loader::FromSaphyrMapping;
use crate::utils::format_scalar;
use crate::utils::saphyr_yaml_string;

/// A TypedSchema is a subset of YamlSchema that has a `type:`
#[derive(Debug, PartialEq)]
pub enum TypedSchema {
    /// `type: null`
    Null,
    /// `type: array`
    Array(ArraySchema),
    /// `type: boolean`
    BooleanSchema,
    /// `type: integer`
    Integer(IntegerSchema),
    /// `type: number`
    Number(NumberSchema),
    /// `type: object`
    Object(Box<ObjectSchema>),
    /// `type: string`
    String(StringSchema),
}

impl TypedSchema {
    pub fn for_yaml_value(value: &saphyr::Yaml) -> Result<TypedSchema> {
        match value {
            saphyr::Yaml::Value(scalar) => match scalar {
                saphyr::Scalar::Null => Ok(TypedSchema::Null),
                saphyr::Scalar::String(s) => TypedSchema::for_type_string(s),
                _ => panic!("Unknown type: {value:?}"),
            },
            _ => panic!("Unknown type: {value:?}"),
        }
    }

    pub fn for_type_string(r#type: &str) -> Result<TypedSchema> {
        match r#type {
            "array" => Ok(TypedSchema::Array(ArraySchema::default())),
            "boolean" => Ok(TypedSchema::BooleanSchema),
            "integer" => Ok(TypedSchema::Integer(IntegerSchema::default())),
            "number" => Ok(TypedSchema::Number(NumberSchema::default())),
            "object" => Ok(TypedSchema::Object(Box::default())),
            "string" => Ok(TypedSchema::String(StringSchema::default())),
            _ => panic!("Unknown type: {type}"),
        }
    }
}

impl TryFrom<&MarkedYaml<'_>> for TypedSchema {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml<'_>) -> crate::Result<Self> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            let type_key = MarkedYaml::value_from_str("type");
            if mapping.contains_key(&type_key) {
                let value = mapping.get(&type_key).unwrap();
                match &value.data {
                    YamlData::Value(scalar) => match scalar {
                        Scalar::String(s) => match s.as_ref() {
                            "array" => {
                                let array_schema = ArraySchema::from_annotated_mapping(mapping)?;
                                Ok(TypedSchema::Array(array_schema))
                            }
                            "boolean" => Ok(TypedSchema::BooleanSchema),
                            "integer" => {
                                let integer_schema: IntegerSchema = marked_yaml.try_into()?;
                                Ok(TypedSchema::Integer(integer_schema))
                            }
                            "number" => {
                                let number_schema: NumberSchema = marked_yaml.try_into()?;
                                Ok(TypedSchema::Number(number_schema))
                            }
                            "object" => {
                                let object_schema: ObjectSchema = marked_yaml.try_into()?;
                                Ok(TypedSchema::Object(Box::new(object_schema)))
                            }
                            "string" => {
                                let string_schema: StringSchema = marked_yaml.try_into()?;
                                Ok(TypedSchema::String(string_schema))
                            }
                            s => Err(unsupported_type!(s.to_string())),
                        },
                        saphyr::Scalar::Null => Ok(TypedSchema::Null),
                        v => Err(unsupported_type!(
                            "Expected a string value for 'type:', but got: {}",
                            format_scalar(v)
                        )),
                    },
                    v => Err(expected_scalar!("Expected scalar type, but got: {:#?}", v)),
                }
            } else {
                Err(generic_error!(
                    "No type key found in mapping: {:#?}",
                    mapping
                ))
            }
        } else {
            Err(expected_mapping!(marked_yaml))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for TypedSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let type_key = MarkedYaml::value_from_str("type");
        if mapping.contains_key(&type_key) {
            let value = mapping.get(&type_key).unwrap();
            match &value.data {
                YamlData::Value(scalar) => match scalar {
                    Scalar::String(s) => match s.as_ref() {
                        "array" => {
                            let array_schema = ArraySchema::from_annotated_mapping(mapping)?;
                            Ok(TypedSchema::Array(array_schema))
                        }
                        "boolean" => Ok(TypedSchema::BooleanSchema),
                        "integer" => {
                            let integer_schema: IntegerSchema = value.try_into()?;
                            Ok(TypedSchema::Integer(integer_schema))
                        }
                        "number" => {
                            let number_schema: NumberSchema = value.try_into()?;
                            Ok(TypedSchema::Number(number_schema))
                        }
                        "object" => {
                            let object_schema: ObjectSchema = value.try_into()?;
                            Ok(TypedSchema::Object(Box::new(object_schema)))
                        }
                        "string" => {
                            let string_schema: StringSchema = value.try_into()?;
                            Ok(TypedSchema::String(string_schema))
                        }
                        s => Err(unsupported_type!(s.to_string())),
                    },
                    saphyr::Scalar::Null => Ok(TypedSchema::Null),
                    v => Err(unsupported_type!(
                        "Expected a string value for 'type:', but got: {}",
                        format_scalar(v)
                    )),
                },
                v => Err(expected_scalar!("Expected scalar type, but got: {:#?}", v)),
            }
        } else {
            Err(generic_error!(
                "No type key found in mapping: {:#?}",
                mapping
            ))
        }
    }
}

impl std::fmt::Display for TypedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedSchema::Array(a) => write!(f, "{a}"),
            TypedSchema::BooleanSchema => write!(f, "type: boolean"),
            TypedSchema::Null => write!(f, "type: null"),
            TypedSchema::Integer(i) => write!(f, "{i}"),
            TypedSchema::Number(n) => write!(f, "{n}"),
            TypedSchema::Object(o) => write!(f, "{o}"),
            TypedSchema::String(s) => write!(f, "{s}"),
        }
    }
}

impl Validator for TypedSchema {
    fn validate(&self, context: &crate::Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[TypedSchema] self: {self}");
        debug!("[TypedSchema] Validating value: {value:?}");
        match self {
            TypedSchema::Array(a) => a.validate(context, value),
            TypedSchema::BooleanSchema => Ok(()),
            TypedSchema::Null => {
                if !value.data.is_null() {
                    context.add_error(value, format!("Expected null, but got: {value:?}"));
                }
                Ok(())
            }
            TypedSchema::Integer(i) => i.validate(context, value),
            TypedSchema::Number(n) => n.validate(context, value),
            TypedSchema::Object(o) => o.validate(context, value),
            TypedSchema::String(s) => s.validate(context, value),
        }
    }
}

impl FromSaphyrMapping<TypedSchema> for TypedSchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> Result<TypedSchema> {
        let type_key = saphyr_yaml_string("type");
        if mapping.contains_key(&type_key) {
            let value = mapping.get(&type_key).unwrap();
            match value {
                saphyr::Yaml::Value(scalar) => match scalar {
                    saphyr::Scalar::String(s) => match s.as_ref() {
                        "array" => {
                            let array_schema = ArraySchema::from_mapping(mapping)?;
                            Ok(TypedSchema::Array(array_schema))
                        }
                        "boolean" => Ok(TypedSchema::BooleanSchema),
                        "integer" => {
                            let integer_schema = IntegerSchema::from_mapping(mapping)?;
                            Ok(TypedSchema::Integer(integer_schema))
                        }
                        "number" => {
                            let number_schema = NumberSchema::from_mapping(mapping)?;
                            Ok(TypedSchema::Number(number_schema))
                        }
                        "object" => {
                            let object_schema = ObjectSchema::from_mapping(mapping)?;
                            Ok(TypedSchema::Object(Box::new(object_schema)))
                        }
                        "string" => {
                            let string_schema = StringSchema::from_mapping(mapping)?;
                            Ok(TypedSchema::String(string_schema))
                        }
                        s => Err(unsupported_type!(s.to_string())),
                    },
                    saphyr::Scalar::Null => Ok(TypedSchema::Null),
                    v => Err(unsupported_type!(
                        "Expected a string value for 'type:', but got: {}",
                        format_scalar(v)
                    )),
                },
                v => Err(expected_scalar!("Expected scalar type, but got: {:#?}", v)),
            }
        } else {
            Err(generic_error!(
                "No type key found in mapping: {:#?}",
                mapping
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode;

    use crate::{ConstValue, Context};

    use super::*;

    #[test]
    fn test_typed_schema_try_from_type_null() {
        let doc = MarkedYaml::load_from_str("type: null").unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        assert_eq!(typed_schema, TypedSchema::Null);
    }

    #[test]
    fn test_typed_schema_try_from_type_string() {
        let doc = MarkedYaml::load_from_str("type: string").unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        assert_eq!(typed_schema, TypedSchema::String(StringSchema::default()));
    }

    #[test]
    fn test_typed_schema_with_enum() {
        let yaml = r#"
        type: string
        enum:
            - "foo"
            - "bar"
        "#;
        println!("yaml: {yaml}");
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        println!("typed_schema: {typed_schema:?}");
        assert!(matches!(typed_schema, TypedSchema::String(_)));
        let TypedSchema::String(string_schema) = typed_schema else {
            panic!("Expected TypedSchema::String, but got: {typed_schema:?}");
        };
        assert_eq!(
            string_schema.base.r#enum,
            Some(vec![ConstValue::string("foo"), ConstValue::string("bar"),])
        );
        let context = Context::default();
        string_schema
            .validate(&context, &MarkedYaml::value_from_str("foo"))
            .expect("Expected no errors");
        println!("context: {context:#?}");
        assert!(!context.has_errors());
        string_schema
            .validate(&context, &MarkedYaml::value_from_str("baz"))
            .expect("Expected no errors");
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        println!("errors: {errors:?}");
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.first().unwrap().error,
            "String is not in enum: [\"foo\", \"bar\"]"
        );
    }
}
