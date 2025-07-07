use crate::Result;
use crate::Validator;
/// The schemas defined in the YAML schema language
use log::debug;
use saphyr::{AnnotatedMapping, MarkedYaml, Scalar, YamlData};

mod any_of;
mod array;
mod bool_or_typed;
mod r#const;
mod r#enum;
mod integer;
mod not;
mod number;
mod object;
mod one_of;
mod string;

use crate::loader::{FromAnnotatedMapping, FromSaphyrMapping};
use crate::utils::{format_marker, format_scalar, saphyr_yaml_string};
pub use any_of::AnyOfSchema;
pub use array::ArraySchema;
pub use bool_or_typed::BoolOrTypedSchema;
pub use integer::IntegerSchema;
pub use not::NotSchema;
pub use number::NumberSchema;
pub use object::ObjectSchema;
pub use one_of::OneOfSchema;
pub use r#const::ConstSchema;
pub use r#enum::EnumSchema;
pub use string::StringSchema;

/// A TypedSchema is a subset of YamlSchema that has a `type:`
#[derive(Debug, PartialEq)]
pub enum TypedSchema {
    Null,
    Array(ArraySchema),        // `type: array`
    BooleanSchema,             // `type: boolean`
    Integer(IntegerSchema),    // `type: integer`
    Number(NumberSchema),      // `type: number`
    Object(Box<ObjectSchema>), // `type: object`
    String(StringSchema),      // `type: string`
}

/// A type value is either a string or an array of strings
#[derive(Debug, PartialEq)]
pub enum TypeValue<'a> {
    Single(saphyr::Yaml<'a>),
    Array(Vec<String>),
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

    fn try_from(marked_yaml: &MarkedYaml<'_>) -> std::result::Result<Self, Self::Error> {
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
            Err(generic_error!(
                "{} expected mapping, got: {:?}",
                format_marker(&marked_yaml.span.start),
                marked_yaml
            ))
        }
    }
}

impl FromAnnotatedMapping<TypedSchema> for TypedSchema {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> Result<Self> {
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
