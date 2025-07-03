use std::rc::Rc;

use hashlink::LinkedHashMap;
use saphyr::LoadableYamlNode;

pub mod engine;
#[macro_use]
pub mod error;
pub mod loader;
pub mod reference;
pub mod schemas;
pub mod utils;
pub mod validation;

pub use engine::Engine;
pub use error::Error;
pub use reference::Reference;
pub use schemas::AnyOfSchema;
pub use schemas::ArraySchema;
pub use schemas::BoolOrTypedSchema;
pub use schemas::ConstSchema;
pub use schemas::EnumSchema;
pub use schemas::IntegerSchema;
pub use schemas::NotSchema;
pub use schemas::NumberSchema;
pub use schemas::ObjectSchema;
pub use schemas::OneOfSchema;
pub use schemas::StringSchema;
pub use validation::Context;
pub use validation::Validator;

use schemas::TypedSchema;

// Returns the library version, which reflects the crate version
pub fn version() -> String {
    clap::crate_version!().to_string()
}

// Alias for std::result::Result<T, yaml_schema::Error>
pub type Result<T> = std::result::Result<T, Error>;

/// A RootSchema is a YamlSchema document
#[derive(Debug, Default)]
pub struct RootSchema {
    pub id: Option<String>,
    pub meta_schema: Option<String>,
    pub defs: Option<LinkedHashMap<String, YamlSchema>>,
    pub schema: Rc<YamlSchema>,
}

impl RootSchema {
    /// Create a new RootSchema with a YamlSchema
    pub fn new(schema: YamlSchema) -> RootSchema {
        RootSchema {
            id: None,
            meta_schema: None,
            defs: None,
            schema: Rc::new(schema),
        }
    }

    /// Create a new RootSchema with a Schema
    pub fn new_with_schema(schema: Schema) -> RootSchema {
        RootSchema::new(YamlSchema::from(schema))
    }

    /// Load a RootSchema from a file
    pub fn load_file(path: &str) -> Result<RootSchema> {
        loader::load_file(path)
    }

    pub fn load_from_str(schema: &str) -> Result<RootSchema> {
        let docs = saphyr::Yaml::load_from_str(schema)?;
        if docs.is_empty() {
            return Ok(RootSchema::new(YamlSchema::empty())); // empty schema
        }
        loader::load_from_doc(docs.first().unwrap())
    }

    pub fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        self.schema.validate(context, value)?;
        Ok(())
    }

    pub fn get_def(&self, name: &str) -> Option<&YamlSchema> {
        if let Some(defs) = &self.defs {
            return defs.get(&name.to_owned());
        }
        None
    }
}

/// A Number is either an integer or a float
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Integer(i64),
    Float(f64),
}

impl Number {
    /// Create a new integer Number
    pub fn integer(value: i64) -> Number {
        Number::Integer(value)
    }

    /// Create a new float Number
    pub fn float(value: f64) -> Number {
        Number::Float(value)
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Integer(v) => write!(f, "{v}"),
            Number::Float(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ConstValue {
    Boolean(bool),
    Null,
    Number(Number),
    String(String),
}

impl ConstValue {
    pub fn boolean(value: bool) -> ConstValue {
        ConstValue::Boolean(value)
    }
    pub fn integer(value: i64) -> ConstValue {
        ConstValue::Number(Number::integer(value))
    }
    pub fn float(value: f64) -> ConstValue {
        ConstValue::Number(Number::float(value))
    }
    pub fn null() -> ConstValue {
        ConstValue::Null
    }
    pub fn string<V: Into<String>>(value: V) -> ConstValue {
        ConstValue::String(value.into())
    }
    pub fn from_saphyr_yaml(value: &saphyr::Yaml) -> ConstValue {
        value.try_into().unwrap()
    }
}

impl<'a> TryFrom<&saphyr::YamlData<'a, saphyr::MarkedYaml<'a>>> for ConstValue {
    type Error = crate::Error;

    fn try_from(value: &saphyr::YamlData<'a, saphyr::MarkedYaml<'a>>) -> Result<Self> {
        match value {
            saphyr::YamlData::Value(scalar) => match scalar {
                saphyr::Scalar::String(s) => Ok(ConstValue::String(s.to_string())),
                saphyr::Scalar::Integer(i) => Ok(ConstValue::Number(Number::integer(*i))),
                saphyr::Scalar::FloatingPoint(o) => {
                    Ok(ConstValue::Number(Number::float(o.into_inner())))
                }
                saphyr::Scalar::Boolean(b) => Ok(ConstValue::Boolean(*b)),
                saphyr::Scalar::Null => Ok(ConstValue::Null),
            },
            v => Err(unsupported_type!(
                "Expected a scalar value, but got: {:?}",
                v
            )),
        }
    }
}

impl TryFrom<&saphyr::Yaml<'_>> for ConstValue {
    type Error = crate::Error;

    fn try_from(value: &saphyr::Yaml) -> Result<Self> {
        match value {
            saphyr::Yaml::Value(scalar) => match scalar {
                saphyr::Scalar::Boolean(b) => Ok(ConstValue::Boolean(*b)),
                saphyr::Scalar::Integer(i) => Ok(ConstValue::Number(Number::integer(*i))),
                saphyr::Scalar::FloatingPoint(o) => {
                    Ok(ConstValue::Number(Number::float(o.into_inner())))
                }
                saphyr::Scalar::String(s) => Ok(ConstValue::String(s.to_string())),
                saphyr::Scalar::Null => Ok(ConstValue::Null),
            },
            v => Err(unsupported_type!(
                "Expected a constant value, but got: {:?}",
                v
            )),
        }
    }
}

impl std::fmt::Display for ConstValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstValue::Boolean(b) => write!(f, "{b} (bool)"),
            ConstValue::Null => write!(f, "null"),
            ConstValue::Number(n) => write!(f, "{n} (number)"),
            ConstValue::String(s) => write!(f, "\"{s}\""),
        }
    }
}

/// YamlSchema is the core of the validation model
#[derive(Debug, Default, PartialEq)]
pub struct YamlSchema {
    pub metadata: Option<LinkedHashMap<String, String>>,
    pub r#ref: Option<Reference>,
    pub schema: Option<Schema>,
}

impl From<Schema> for YamlSchema {
    fn from(schema: Schema) -> Self {
        YamlSchema {
            schema: Some(schema),
            ..Default::default()
        }
    }
}

impl YamlSchema {
    pub fn empty() -> YamlSchema {
        YamlSchema {
            schema: Some(Schema::Empty),
            ..Default::default()
        }
    }

    pub fn null() -> YamlSchema {
        YamlSchema {
            schema: Some(Schema::TypeNull),
            ..Default::default()
        }
    }

    pub fn boolean_literal(value: bool) -> YamlSchema {
        YamlSchema {
            schema: Some(Schema::BooleanLiteral(value)),
            ..Default::default()
        }
    }

    pub fn reference(reference: Reference) -> YamlSchema {
        YamlSchema {
            r#ref: Some(reference),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum Schema {
    #[default]
    Empty, // no value
    BooleanLiteral(bool),   // `true` or `false`
    Const(ConstSchema),     // `const`
    TypeNull,               // `type: null`
    Array(ArraySchema),     // `type: array`
    BooleanSchema,          // `type: boolean`
    Integer(IntegerSchema), // `type: integer`
    Number(NumberSchema),   // `type: number`
    Object(ObjectSchema),   // `type: object`
    String(StringSchema),   // `type: string`
    Enum(EnumSchema),       // `enum`
    AnyOf(AnyOfSchema),     // `anyOf`
    OneOf(OneOfSchema),     // `oneOf`
    Not(NotSchema),         // `not`
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
            Schema::AnyOf(any_of_schema) => {
                write!(f, "{any_of_schema}")
            }
            Schema::OneOf(one_of_schema) => {
                write!(f, "{one_of_schema}")
            }
            Schema::Not(not_schema) => {
                write!(f, "{not_schema}")
            }
            Schema::String(s) => write!(f, "{s}"),
            Schema::Number(n) => write!(f, "{n}"),
            Schema::Object(o) => write!(f, "{o}"),
            Schema::Array(a) => write!(f, "{a}"),
        }
    }
}

impl std::fmt::Display for YamlSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        if let Some(metadata) = &self.metadata {
            write!(f, "metadata: {metadata:?}, ")?;
        }
        if let Some(r#ref) = &self.r#ref {
            r#ref.fmt(f)?;
        }
        if let Some(schema) = &self.schema {
            write!(f, "schema: {schema}")?;
        }
        write!(f, "}}")
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

/// Use the ctor crate to initialize the logger for tests
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
    fn test_const_equality() {
        let i1 = ConstValue::integer(42);
        let i2 = ConstValue::integer(42);
        assert_eq!(i1, i2);

        let s1 = ConstValue::string("NW");
        let s2 = ConstValue::string("NW");
        assert_eq!(s1, s2);
    }
}
