/// The deser module contains code to "deserialize" a YamlSchema validation model from YAML
/// It declares and uses an intermediate `deser::YamlSchema` model
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::format_map;
use crate::format_vec;
use crate::generic_error;
use crate::unsupported_type;
use crate::BoolOrTypedSchema;
use crate::Number;
use crate::Result;

/// Instead of From<deser::YamlSchema>, or rather, Into<T>
pub trait Deser<T>: Sized {
    fn deserialize(&self) -> Result<T>;
}

/// A YamlSchema is either empty, a boolean, a typed schema, or an enum schema
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum YamlSchema {
    #[default]
    Empty,
    Boolean(bool),
    Const(ConstSchema),
    Enum(EnumSchema),
    AnyOf(AnyOfSchema),
    OneOf(OneOfSchema),
    Not(NotSchema),
    // Need to put TypedSchema last, because not specifying `type:`
    // is interpreted as `type: null` (None)
    TypedSchema(Box<TypedSchema>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PropertyNamesValue {
    pub pattern: String,
}

/// A typed schema is a schema that has a type
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TypedSchema {
    pub r#type: serde_yaml::Value,
    // number
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
    // object
    pub properties: Option<HashMap<String, YamlSchema>>,
    pub required: Option<Vec<String>>,
    pub additional_properties: Option<AdditionalProperties>,
    pub pattern_properties: Option<HashMap<String, YamlSchema>>,
    pub property_names: Option<PropertyNamesValue>,
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
    // string
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    // array
    pub items: Option<ArrayItemsValue>,
    pub prefix_items: Option<Vec<YamlSchema>>,
    pub contains: Option<YamlSchema>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ConstSchema {
    pub r#const: serde_yaml::Value,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct EnumSchema {
    pub r#enum: Vec<serde_yaml::Value>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Boolean(bool),
    Type { r#type: serde_yaml::Value },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ArrayItemsValue {
    TypedSchema(Box<TypedSchema>),
    Boolean(bool),
}

impl std::fmt::Display for YamlSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlSchema::Empty => write!(f, "<empty schema>"),
            YamlSchema::Boolean(b) => write!(f, "{}", b),
            YamlSchema::Const(c) => write!(f, "{}", c),
            YamlSchema::Enum(e) => write!(f, "{}", e),
            YamlSchema::AnyOf(any_of_schema) => {
                write!(f, "{}", any_of_schema)
            }
            YamlSchema::OneOf(one_of_schema) => {
                write!(f, "{}", one_of_schema)
            }
            YamlSchema::Not(not_schema) => {
                write!(f, "{}", not_schema)
            }
            YamlSchema::TypedSchema(s) => write!(f, "{}", s),
        }
    }
}

impl YamlSchema {
    pub fn new() -> YamlSchema {
        YamlSchema::Empty
    }

    pub fn const_schema<V>(value: V) -> YamlSchema
    where
        V: Into<serde_yaml::Value>,
    {
        YamlSchema::Const(ConstSchema {
            r#const: value.into(),
        })
    }

    pub fn one_of(schemas: Vec<YamlSchema>) -> YamlSchema {
        YamlSchema::OneOf(OneOfSchema { one_of: schemas })
    }

    pub fn typed_schema(schema: TypedSchema) -> YamlSchema {
        YamlSchema::TypedSchema(Box::new(schema))
    }

    pub fn is_none(&self) -> bool {
        self == &YamlSchema::Empty
    }
}

impl From<&ConstSchema> for crate::ConstSchema {
    fn from(const_schema: &ConstSchema) -> Self {
        Self {
            r#const: const_schema.r#const.clone(),
        }
    }
}

impl From<&EnumSchema> for crate::EnumSchema {
    fn from(enum_schema: &EnumSchema) -> Self {
        Self {
            r#enum: enum_schema.r#enum.clone(),
        }
    }
}

impl From<&AnyOfSchema> for crate::AnyOfSchema {
    fn from(any_of_schema: &AnyOfSchema) -> Self {
        let any_of: Vec<crate::YamlSchema> = any_of_schema
            .any_of
            .iter()
            .map(|s| s.deserialize().unwrap())
            .collect();
        crate::AnyOfSchema { any_of }
    }
}

impl From<&OneOfSchema> for crate::OneOfSchema {
    fn from(one_of_schema: &OneOfSchema) -> Self {
        let one_of: Vec<crate::YamlSchema> = one_of_schema
            .one_of
            .iter()
            .map(|s| s.deserialize().unwrap())
            .collect();
        crate::OneOfSchema { one_of }
    }
}

impl Deser<crate::NotSchema> for NotSchema {
    fn deserialize(&self) -> Result<crate::NotSchema> {
        Ok(crate::NotSchema {
            not: Box::new(self.not.deserialize()?),
        })
    }
}

impl Deser<crate::YamlSchema> for YamlSchema {
    fn deserialize(&self) -> Result<crate::YamlSchema> {
        match &self {
            YamlSchema::Empty => Ok(crate::YamlSchema::Empty),
            YamlSchema::Boolean(b) => Ok(crate::YamlSchema::BooleanLiteral(*b)),
            YamlSchema::Const(c) => Ok(crate::YamlSchema::Const(c.into())),
            YamlSchema::Enum(e) => Ok(crate::YamlSchema::Enum(e.into())),
            YamlSchema::AnyOf(a) => Ok(crate::YamlSchema::AnyOf(a.into())),
            YamlSchema::OneOf(o) => Ok(crate::YamlSchema::OneOf(o.into())),
            YamlSchema::Not(n) => Ok(crate::YamlSchema::Not(n.deserialize()?)),
            YamlSchema::TypedSchema(t) => {
                let typed_schema: crate::TypedSchema = t.deserialize()?;
                Ok(typed_schema.into())
            }
        }
    }
}

impl TypedSchema {
    pub fn null() -> TypedSchema {
        TypedSchema {
            r#type: serde_yaml::Value::Null,
            ..Default::default()
        }
    }

    pub fn string() -> TypedSchema {
        TypedSchema {
            r#type: serde_yaml::Value::String("string".to_string()),
            ..Default::default()
        }
    }

    pub fn number() -> TypedSchema {
        TypedSchema {
            r#type: serde_yaml::Value::String("number".to_string()),
            ..Default::default()
        }
    }

    pub fn object(properties: HashMap<String, YamlSchema>) -> TypedSchema {
        TypedSchema {
            r#type: serde_yaml::Value::String("object".to_string()),
            properties: Some(properties),
            ..Default::default()
        }
    }
}

// We don't need to implement Deser<crate::IntegerSchema> for TypedSchema
// because we don't have any failure modes (yet) for this deserialization
impl From<&TypedSchema> for crate::IntegerSchema {
    fn from(typed_schema: &TypedSchema) -> Self {
        Self {
            multiple_of: typed_schema.multiple_of,
            exclusive_maximum: typed_schema.exclusive_maximum,
            exclusive_minimum: typed_schema.exclusive_minimum,
            maximum: typed_schema.maximum,
            minimum: typed_schema.minimum,
        }
    }
}

// We don't need to implement Deser<crate::NumberSchema> for TypedSchema
// because we don't have any failure modes (yet) for this deserialization
impl From<&TypedSchema> for crate::NumberSchema {
    fn from(typed_schema: &TypedSchema) -> Self {
        Self {
            multiple_of: typed_schema.multiple_of,
            exclusive_maximum: typed_schema.exclusive_maximum,
            exclusive_minimum: typed_schema.exclusive_minimum,
            maximum: typed_schema.maximum,
            minimum: typed_schema.minimum,
        }
    }
}

impl Deser<crate::BoolOrTypedSchema> for TypedSchema {
    fn deserialize(&self) -> Result<crate::BoolOrTypedSchema> {
        Ok(match &self.r#type {
            serde_yaml::Value::Null => crate::BoolOrTypedSchema::Boolean(false),
            serde_yaml::Value::String(s) => {
                let typed_schema = self.deserialize_by_type_string(s.as_str())?;
                crate::BoolOrTypedSchema::TypedSchema(Box::new(typed_schema))
            }
            unknown => {
                return unsupported_type!(
                    "Don't know how to deserialize a type value of: {:?}",
                    unknown
                )
            }
        })
    }
}

impl Deser<crate::TypedSchema> for TypedSchema {
    fn deserialize(&self) -> Result<crate::TypedSchema> {
        Ok(match &self.r#type {
            serde_yaml::Value::Null => crate::TypedSchema::Null,
            serde_yaml::Value::String(s) => self.deserialize_by_type_string(s.as_str())?,
            unknown => {
                return unsupported_type!(
                    "Don't know how to deserialize a type value of: {:?}",
                    unknown
                )
            }
        })
    }
}

impl TypedSchema {
    pub fn deserialize_by_type_string(&self, s: &str) -> Result<crate::TypedSchema> {
        match s {
            "array" => Ok(crate::TypedSchema::Array(self.deserialize()?)),
            "boolean" => Ok(crate::TypedSchema::BooleanSchema),
            "integer" => Ok(crate::TypedSchema::Integer(self.into())),
            "number" => Ok(crate::TypedSchema::Number(self.into())),
            "object" => Ok(crate::TypedSchema::Object(self.deserialize()?)),
            "string" => Ok(crate::TypedSchema::String(self.deserialize()?)),
            unknown => {
                unsupported_type!("Unrecognized type '{}'!", unknown)
            }
        }
    }
}

impl std::fmt::Display for TypedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fields = Vec::new();

        fields.push(format!("type: {:?}", self.r#type));

        if let Some(min) = &self.minimum {
            fields.push(format!("minimum: {}", min));
        }
        if let Some(max) = &self.maximum {
            fields.push(format!("maximum: {}", max));
        }
        if let Some(ex_min) = &self.exclusive_minimum {
            fields.push(format!("exclusiveMinimum: {}", ex_min));
        }
        if let Some(ex_max) = &self.exclusive_maximum {
            fields.push(format!("exclusiveMaximum: {}", ex_max));
        }
        if let Some(mult_of) = &self.multiple_of {
            fields.push(format!("multipleOf: {}", mult_of));
        }
        if let Some(props) = &self.properties {
            fields.push(format!("properties: {}", format_map(props)));
        }
        if let Some(req) = &self.required {
            fields.push(format!("required: {:?}", req));
        }
        if let Some(add_props) = &self.additional_properties {
            fields.push(format!("additionalProperties: {}", add_props));
        }
        if let Some(pattern_props) = &self.pattern_properties {
            fields.push(format!("patternProperties: {}", format_map(pattern_props)));
        }
        if let Some(min_len) = &self.min_length {
            fields.push(format!("minLength: {}", min_len));
        }
        if let Some(max_len) = &self.max_length {
            fields.push(format!("maxLength: {}", max_len));
        }
        if let Some(pattern) = &self.pattern {
            fields.push(format!("pattern: {}", pattern));
        }
        if let Some(items) = &self.items {
            fields.push(format!("items: {}", items));
        }
        if let Some(prefix_items) = &self.prefix_items {
            fields.push(format!("prefixItems: {}", format_vec(prefix_items)));
        }
        if let Some(contains) = &self.contains {
            fields.push(format!("contains: {}", contains));
        }

        write!(f, "TypedSchema {{ {} }}", fields.join(", "))
    }
}

impl Deser<crate::BoolOrTypedSchema> for ArrayItemsValue {
    fn deserialize(&self) -> Result<crate::BoolOrTypedSchema> {
        match self {
            ArrayItemsValue::Boolean(b) => Ok(crate::BoolOrTypedSchema::Boolean(*b)),
            ArrayItemsValue::TypedSchema(t) => Ok(t.deserialize()?),
        }
    }
}

impl Deser<crate::ArraySchema> for TypedSchema {
    fn deserialize(&self) -> Result<crate::ArraySchema> {
        let items: Option<BoolOrTypedSchema> =
            self.items.as_ref().map(|i| i.deserialize().unwrap());
        let prefix_items: Option<Vec<Box<crate::YamlSchema>>> =
            self.prefix_items.as_ref().map(|prefix_items| {
                prefix_items
                    .iter()
                    .map(|y: &crate::deser::YamlSchema| Box::new(y.deserialize().unwrap()))
                    .collect()
            });
        let contains: Option<Box<crate::YamlSchema>> = self
            .contains
            .as_ref()
            .map(|c| Box::new(c.deserialize().unwrap()));
        Ok(crate::ArraySchema {
            items,
            prefix_items,
            contains,
        })
    }
}

impl Deser<crate::StringSchema> for TypedSchema {
    fn deserialize(&self) -> Result<crate::StringSchema> {
        let pattern = match &self.pattern {
            None => None,
            Some(p) => {
                if let Ok(re) = regex::Regex::new(p) {
                    Some(re)
                } else {
                    return generic_error!("Invalid regular expression pattern: {}", p);
                }
            }
        };
        Ok(crate::StringSchema {
            min_length: self.min_length,
            max_length: self.max_length,
            pattern,
        })
    }
}

impl Deser<crate::ObjectSchema> for TypedSchema {
    fn deserialize(&self) -> Result<crate::ObjectSchema> {
        let properties = self.properties.as_ref().map(|p| {
            p.iter()
                .map(|(k, v)| (k.clone(), v.deserialize().unwrap()))
                .collect()
        });
        Ok(crate::ObjectSchema {
            properties,
            required: self.required.clone(),
            additional_properties: self.additional_properties.as_ref().map(|a| match a {
                AdditionalProperties::Boolean(b) => crate::schemas::BoolOrTypedSchema::Boolean(*b),
                AdditionalProperties::Type { r#type } => {
                    let typed_schema = crate::TypedSchema::for_yaml_value(r#type).unwrap();
                    crate::schemas::BoolOrTypedSchema::TypedSchema(Box::new(typed_schema))
                }
            }),
            pattern_properties: self.pattern_properties.as_ref().map(|p| {
                p.iter()
                    .map(|(k, v)| (k.clone(), v.deserialize().unwrap()))
                    .collect()
            }),
            // if Some(PropertyNamesValue) => Some(p.pattern.clone()),
            property_names: self.property_names.as_ref().map(|p| p.pattern.clone()),
            min_properties: self.min_properties,
            max_properties: self.max_properties,
        })
    }
}

impl ConstSchema {
    pub fn new<V>(value: V) -> ConstSchema
    where
        V: Into<serde_yaml::Value>,
    {
        ConstSchema {
            r#const: value.into(),
        }
    }

    pub fn null() -> ConstSchema {
        ConstSchema {
            r#const: serde_yaml::Value::Null,
        }
    }

    pub fn string<V>(value: V) -> ConstSchema
    where
        V: Into<String>,
    {
        ConstSchema {
            r#const: serde_yaml::Value::String(value.into()),
        }
    }
}

impl std::fmt::Display for ConstSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Const {:?}", self.r#const)
    }
}

impl EnumSchema {
    pub fn new<V>(values: Vec<V>) -> EnumSchema
    where
        V: Into<serde_yaml::Value>,
    {
        let values = values.into_iter().map(|v| v.into()).collect();
        EnumSchema { r#enum: values }
    }
}

impl std::fmt::Display for EnumSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Enum {:?}", self.r#enum)
    }
}

impl std::fmt::Display for AdditionalProperties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdditionalProperties::Boolean(b) => write!(f, "additionalProperties: {}", b),
            AdditionalProperties::Type { r#type } => {
                write!(f, "additionalProperties: {:?}", r#type)
            }
        }
    }
}

impl std::fmt::Display for ArrayItemsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArrayItemsValue::TypedSchema(s) => write!(f, "{}", s),
            ArrayItemsValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

/// The `anyOf` schema is a schema that matches if any of the schemas in the `anyOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnyOfSchema {
    pub any_of: Vec<YamlSchema>,
}

impl std::fmt::Display for AnyOfSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "anyOf: {}", format_vec(&self.any_of))
    }
}

/// The `oneOf` schema is a schema that matches if one, and only one of the schemas in the `oneOf` array match.
/// The schemas are tried in order, and the first match is used. If no match is found, an error is added
/// to the context.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OneOfSchema {
    pub one_of: Vec<YamlSchema>,
}

impl std::fmt::Display for OneOfSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "oneOf: {}", format_vec(&self.one_of))
    }
}

/// The `not` ` keyword declares that an instance validates if it doesn't validate against the given subschema.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotSchema {
    pub not: Box<YamlSchema>,
}

impl std::fmt::Display for NotSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not: {}", self.not)
    }
}

// Initialize the logger for tests
#[cfg(test)]
#[ctor::ctor]
fn init() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format_target(false)
        .format_timestamp_secs()
        .target(env_logger::Target::Stdout)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_schema() {
        let schema: YamlSchema = serde_yaml::from_str("").unwrap();
        assert!(schema.is_none());
    }

    #[test]
    fn test_parse_false_schema() {
        let schema: YamlSchema = serde_yaml::from_str("false").unwrap();
        let expected = YamlSchema::Boolean(false);
        assert_eq!(expected, schema);
    }

    #[test]
    fn test_parse_true_schema() {
        let schema: YamlSchema = serde_yaml::from_str("true").unwrap();
        let expected = YamlSchema::Boolean(true);
        assert_eq!(expected, schema);
    }

    #[test]
    fn test_parse_type_string_schema() {
        let schema: YamlSchema = serde_yaml::from_str("type: string").unwrap();
        let expected = YamlSchema::TypedSchema(Box::new(TypedSchema::string()));
        assert_eq!(expected, schema);
    }

    #[test]
    fn test_null_schema() {
        let schema: YamlSchema = serde_yaml::from_str("type: null").unwrap();
        match schema {
            YamlSchema::TypedSchema(s) => {
                assert_eq!(s.r#type, serde_yaml::Value::Null);
            }
            _ => panic!("Expected a TypedSchema"),
        }
    }

    #[test]
    fn test_number_schema() {
        let yaml = "
        type: number
        multipleOf: 5
        ";
        let schema: YamlSchema = serde_yaml::from_str(yaml).unwrap();
        println!("{}", schema);
    }

    #[test]
    fn test_one_of_schema() {
        let yaml = "
        oneOf:
        - type: number
          multipleOf: 5
        - type: number
          multipleOf: 3
        ";
        let schema: YamlSchema = serde_yaml::from_str(yaml).unwrap();
        println!("{}", schema);
    }

    #[test]
    fn test_deserialize_type_null() {
        let yaml = "
        type: null
        ";
        let schema: YamlSchema = serde_yaml::from_str(yaml).unwrap();
        println!("{}", schema);
        let yaml_schema = schema.deserialize().unwrap();
        println!("{}", yaml_schema);
    }

    #[test]
    fn test_typed_schema_can_deserialize_to_string_schema() {
        let typed_schema: TypedSchema = serde_yaml::from_str(
            r#"
            type: string
        "#,
        )
        .unwrap();
        let string_schema: crate::StringSchema = typed_schema.deserialize().unwrap();
        println!("{}", string_schema);
    }

    #[test]
    fn test_typed_schema_can_deserialize_to_object_schema() {
        let typed_schema: TypedSchema = serde_yaml::from_str(
            r#"
            type: object
            properties:
                foo:
                    type: string
                bar:
                    type: number
        "#,
        )
        .unwrap();
        let object_schema: crate::ObjectSchema = typed_schema.deserialize().unwrap();
        println!("{}", object_schema);
    }
}
