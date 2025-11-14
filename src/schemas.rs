// The schemas defined in the YAML schema language

use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::YamlData;

mod all_of;
mod any_of;
mod array;
mod base;
mod bool_or_typed;
mod r#const;
mod r#enum;
mod integer;
mod not;
mod number;
mod object;
mod one_of;
mod string;
mod typed_schema;
mod yaml_schema;

pub use all_of::AllOfSchema;
pub use any_of::AnyOfSchema;
pub use array::ArraySchema;
pub use base::BaseSchema;
pub use bool_or_typed::BoolOrTypedSchema;
pub use r#const::ConstSchema;
pub use r#enum::EnumSchema;
pub use integer::IntegerSchema;
pub use not::NotSchema;
pub use number::NumberSchema;
pub use object::ObjectSchema;
pub use one_of::OneOfSchema;
pub use string::StringSchema;
pub use typed_schema::TypedSchema;
pub use yaml_schema::YamlSchema;

use crate::Result;

#[derive(Debug, Default, PartialEq)]
pub enum Schema {
    #[default]
    Empty, // no value
    BooleanLiteral(bool),      // `true` or `false`
    Const(ConstSchema),        // `const`
    TypeNull,                  // `type: null`
    Array(ArraySchema),        // `type: array`
    BooleanSchema,             // `type: boolean`
    Integer(IntegerSchema),    // `type: integer`
    Number(NumberSchema),      // `type: number`
    Object(Box<ObjectSchema>), // `type: object`
    String(StringSchema),      // `type: string`
    Enum(EnumSchema),          // `enum`
    AllOf(AllOfSchema),        // `allOf`
    AnyOf(AnyOfSchema),        // `anyOf`
    OneOf(OneOfSchema),        // `oneOf`
    Not(NotSchema),            // `not`
}

impl Schema {
    pub fn object(schema: ObjectSchema) -> Schema {
        Schema::Object(Box::new(schema))
    }
}

impl TryFrom<&MarkedYaml<'_>> for Schema {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml) -> Result<Schema> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            if mapping.is_empty() {
                Err(generic_error!("Empty mapping"))
            } else if mapping.contains_key(&MarkedYaml::value_from_str("type")) {
                let typed_schema: TypedSchema = marked_yaml.try_into()?;
                Ok(typed_schema.into())
            } else if mapping.contains_key(&MarkedYaml::value_from_str("enum")) {
                let enum_schema = EnumSchema::try_from(mapping)?;
                Ok(Schema::Enum(enum_schema))
            } else if mapping.contains_key(&MarkedYaml::value_from_str("const")) {
                let const_schema = ConstSchema::try_from(mapping)?;
                Ok(Schema::Const(const_schema))
            } else if mapping.contains_key(&MarkedYaml::value_from_str("anyOf")) {
                let any_of_schema = AnyOfSchema::try_from(mapping)?;
                Ok(Schema::AnyOf(any_of_schema))
            } else if mapping.contains_key(&MarkedYaml::value_from_str("oneOf")) {
                let one_of_schema = marked_yaml.try_into()?;
                Ok(Schema::OneOf(one_of_schema))
            } else if mapping.contains_key(&MarkedYaml::value_from_str("not")) {
                let not_schema = NotSchema::try_from(mapping)?;
                Ok(Schema::Not(not_schema))
            } else {
                Err(generic_error!(
                    "(Schema) Don't know how to construct schema: {:?}",
                    mapping
                ))
            }
        } else {
            Err(expected_mapping!(marked_yaml))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for Schema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> Result<Self> {
        if mapping.is_empty() {
            Err(generic_error!("Empty mapping"))
        } else if mapping.contains_key(&MarkedYaml::value_from_str("type")) {
            let typed_schema: TypedSchema = mapping.try_into()?;
            Ok(typed_schema.into())
        } else if mapping.contains_key(&MarkedYaml::value_from_str("enum")) {
            let enum_schema = EnumSchema::try_from(mapping)?;
            Ok(Schema::Enum(enum_schema))
        } else if mapping.contains_key(&MarkedYaml::value_from_str("const")) {
            let const_schema = ConstSchema::try_from(mapping)?;
            Ok(Schema::Const(const_schema))
        } else if mapping.contains_key(&MarkedYaml::value_from_str("allOf")) {
            let all_of_schema = AllOfSchema::try_from(mapping)?;
            Ok(Schema::AllOf(all_of_schema))
        } else if mapping.contains_key(&MarkedYaml::value_from_str("anyOf")) {
            let any_of_schema = AnyOfSchema::try_from(mapping)?;
            Ok(Schema::AnyOf(any_of_schema))
        } else if mapping.contains_key(&MarkedYaml::value_from_str("oneOf")) {
            let one_of_schema = OneOfSchema::try_from(mapping)?;
            Ok(Schema::OneOf(one_of_schema))
        } else if mapping.contains_key(&MarkedYaml::value_from_str("not")) {
            let not_schema = NotSchema::try_from(mapping)?;
            Ok(Schema::Not(not_schema))
        } else {
            Err(generic_error!(
                "Don't know how to construct schema: {:#?}",
                mapping
            ))
        }
    }
}

impl std::fmt::Display for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Schema::Empty => write!(f, "<empty schema>"),
            Schema::TypeNull => write!(f, "type: null"),
            Schema::BooleanLiteral(b) => write!(f, "{b}"),
            Schema::BooleanSchema => write!(f, "type: boolean"),
            Schema::Const(c) => write!(f, "{c}"),
            Schema::Enum(e) => write!(f, "{e}"),
            Schema::Integer(i) => write!(f, "{i}"),
            Schema::AllOf(all_of_schema) => write!(f, "{all_of_schema}"),
            Schema::AnyOf(any_of_schema) => write!(f, "{any_of_schema}"),
            Schema::OneOf(one_of_schema) => write!(f, "{one_of_schema}"),
            Schema::Not(not_schema) => write!(f, "{not_schema}"),
            Schema::String(s) => write!(f, "{s}"),
            Schema::Number(n) => write!(f, "{n}"),
            Schema::Object(o) => write!(f, "{o}"),
            Schema::Array(a) => write!(f, "{a}"),
        }
    }
}

/// Converts (upcast) a TypedSchema to a YamlSchema
/// Since a YamlSchema is a superset of a TypedSchema, this is a lossless conversion
impl From<TypedSchema> for Schema {
    fn from(schema: TypedSchema) -> Self {
        match schema {
            TypedSchema::Array(array_schema) => Schema::Array(array_schema),
            TypedSchema::BooleanSchema => Schema::BooleanSchema,
            TypedSchema::Null => Schema::TypeNull,
            TypedSchema::Integer(integer_schema) => Schema::Integer(integer_schema),
            TypedSchema::Number(number_schema) => Schema::Number(number_schema),
            TypedSchema::Object(object_schema) => Schema::Object(object_schema),
            TypedSchema::String(string_schema) => Schema::String(string_schema),
        }
    }
}
