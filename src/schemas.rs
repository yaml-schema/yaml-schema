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
pub use typed_schema::TypedSchemaType;
pub use yaml_schema::YamlSchema;

use crate::Result;

pub trait SchemaMetadata {
    fn get_accepted_keys() -> &'static [&'static str];
}

/// The inner Schema of a YamlSchema
#[derive(Debug, Default, PartialEq)]
pub enum Schema {
    #[default]
    Empty, // no value
    BooleanLiteral(bool), // `true` or `false`
    Const(ConstSchema),   // `const`
    Typed(TypedSchema),   // `type`
    Enum(EnumSchema),     // `enum`
    AllOf(AllOfSchema),   // `allOf`
    AnyOf(AnyOfSchema),   // `anyOf`
    OneOf(OneOfSchema),   // `oneOf`
    Not(NotSchema),       // `not`
}

impl Schema {
    pub fn typed_object(schema: ObjectSchema) -> Schema {
        Schema::Typed(TypedSchema::object(schema))
    }

    pub fn typed_array(schema: ArraySchema) -> Schema {
        Schema::Typed(TypedSchema::array(schema))
    }

    pub fn typed_boolean() -> Schema {
        Schema::Typed(TypedSchema::boolean())
    }

    pub fn typed_integer(schema: IntegerSchema) -> Schema {
        Schema::Typed(TypedSchema::integer(schema))
    }

    pub fn typed_number(schema: NumberSchema) -> Schema {
        Schema::Typed(TypedSchema::number(schema))
    }

    pub fn typed_string(schema: StringSchema) -> Schema {
        Schema::Typed(TypedSchema::string(schema))
    }

    pub fn typed_null() -> Schema {
        Schema::Typed(TypedSchema::null())
    }

    pub fn is_typed(&self) -> bool {
        matches!(self, Schema::Typed(_))
    }

    pub fn as_typed_schema(&self) -> Result<&TypedSchema> {
        if let Self::Typed(typed_schema) = self {
            Ok(typed_schema)
        } else {
            Err(generic_error!("Schema is not a typed schema"))
        }
    }

    pub fn is_object(&self) -> bool {
        if let Self::Typed(typed_schema) = self
            && typed_schema.r#type.len() == 1
        {
            return matches!(
                typed_schema.r#type.first().unwrap(),
                TypedSchemaType::Object(_)
            );
        }
        false
    }
}

impl TryFrom<&MarkedYaml<'_>> for Schema {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml) -> Result<Schema> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            Ok(Schema::try_from(mapping)?)
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
            Ok(Schema::Typed(typed_schema))
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
            Schema::BooleanLiteral(b) => write!(f, "{b}"),
            Schema::Const(c) => write!(f, "{c}"),
            Schema::Enum(e) => write!(f, "{e}"),
            Schema::AllOf(all_of_schema) => write!(f, "{all_of_schema}"),
            Schema::AnyOf(any_of_schema) => write!(f, "{any_of_schema}"),
            Schema::OneOf(one_of_schema) => write!(f, "{one_of_schema}"),
            Schema::Not(not_schema) => write!(f, "{not_schema}"),
            Schema::Typed(typed_schema) => write!(f, "{typed_schema}"),
        }
    }
}
