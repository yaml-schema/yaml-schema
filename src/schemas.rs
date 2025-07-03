/// The schemas defined in the YAML schema language
use log::debug;
use std::fmt;

use crate::Result;
use crate::Validator;

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
    Array(ArraySchema),     // `type: array`
    BooleanSchema,          // `type: boolean`
    Integer(IntegerSchema), // `type: integer`
    Number(NumberSchema),   // `type: number`
    Object(ObjectSchema),   // `type: object`
    String(StringSchema),   // `type: string`
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
            "object" => Ok(TypedSchema::Object(ObjectSchema::default())),
            "string" => Ok(TypedSchema::String(StringSchema::default())),
            _ => panic!("Unknown type: {type}"),
        }
    }
}

impl fmt::Display for TypedSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
