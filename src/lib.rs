use hashlink::LinkedHashMap;
use saphyr::{AnnotatedMapping, LoadableYamlNode, MarkedYaml, Scalar, YamlData};
use std::rc::Rc;

pub mod engine;
#[macro_use]
pub mod error;
pub mod codegen;
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
pub use schemas::YamlSchema;
pub use validation::Context;
pub use validation::Validator;

use crate::loader::FromAnnotatedMapping;
use crate::utils::format_marker;
use schemas::TypedSchema;

// Returns the library version, which reflects the crate version
pub fn version() -> String {
    clap::crate_version!().to_string()
}

// Alias for std::result::Result<T, yaml_schema::Error>
pub type Result<T> = std::result::Result<T, Error>;

/// A RootSchema represents the root document in a schema file, and can include additional
/// fields not present in the 'base' YamlSchema
#[derive(Debug, Default, PartialEq)]
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

    /// Builder pattern for RootSchema
    pub fn builder() -> RootSchemaBuilder {
        RootSchemaBuilder::new()
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
        let docs = MarkedYaml::load_from_str(schema)?;
        if docs.is_empty() {
            return Ok(RootSchema::new(YamlSchema::empty())); // empty schema
        }
        loader::load_from_doc(docs.first().unwrap())
    }

    pub fn validate(&self, context: &Context, value: &MarkedYaml) -> Result<()> {
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

pub struct RootSchemaBuilder(RootSchema);

impl Default for RootSchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RootSchemaBuilder {
    /// Construct a RootSchemaBuilder
    pub fn new() -> Self {
        Self(RootSchema::default())
    }

    pub fn build(&mut self) -> RootSchema {
        std::mem::take(&mut self.0)
    }

    pub fn id<S: Into<String>>(&mut self, id: S) -> &mut Self {
        self.0.id = Some(id.into());
        self
    }

    pub fn meta_schema<S: Into<String>>(&mut self, meta_schema: S) -> &mut Self {
        self.0.meta_schema = Some(meta_schema.into());
        self
    }

    pub fn defs(&mut self, defs: LinkedHashMap<String, YamlSchema>) -> &mut Self {
        self.0.defs = Some(defs);
        self
    }

    pub fn schema(&mut self, schema: YamlSchema) -> &mut Self {
        self.0.schema = Rc::new(schema);
        self
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

impl TryFrom<&MarkedYaml<'_>> for Number {
    type Error = Error;
    fn try_from(value: &MarkedYaml) -> Result<Number> {
        if let YamlData::Value(scalar) = &value.data {
            match scalar {
                Scalar::Integer(i) => Ok(Number::integer(*i)),
                Scalar::FloatingPoint(o) => Ok(Number::float(o.into_inner())),
                _ => Err(generic_error!(
                    "{} Expected type: integer or float, but got: {:?}",
                    format_marker(&value.span.start),
                    value
                )),
            }
        } else {
            Err(generic_error!(
                "{} Expected scalar, but got: {:?}",
                format_marker(&value.span.start),
                value
            ))
        }
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

/// A ConstValue is similar to a saphyr::Scalar, but for validating "number" types
/// we treat integers and floating point values as 'fungible'
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

impl TryFrom<&Scalar<'_>> for ConstValue {
    type Error = crate::Error;

    fn try_from(scalar: &Scalar) -> std::result::Result<ConstValue, Self::Error> {
        match scalar {
            Scalar::Null => Ok(ConstValue::Null),
            Scalar::Boolean(b) => Ok(ConstValue::Boolean(*b)),
            Scalar::Integer(i) => Ok(ConstValue::Number(Number::integer(*i))),
            Scalar::FloatingPoint(o) => Ok(ConstValue::Number(Number::float(o.into_inner()))),
            Scalar::String(s) => Ok(ConstValue::String(s.to_string())),
        }
    }
}

impl<'a> TryFrom<&YamlData<'a, MarkedYaml<'a>>> for ConstValue {
    type Error = crate::Error;

    fn try_from(value: &YamlData<'a, MarkedYaml<'a>>) -> Result<Self> {
        match value {
            YamlData::Value(scalar) => scalar.try_into(),
            v => Err(generic_error!("Expected a scalar value, but got: {:?}", v)),
        }
    }
}

impl<'a> TryFrom<&MarkedYaml<'a>> for ConstValue {
    type Error = crate::Error;
    fn try_from(value: &MarkedYaml<'a>) -> Result<ConstValue> {
        match (&value.data).try_into() {
            Ok(r) => Ok(r),
            _ => Err(generic_error!(
                "{} Expected a scalar value, but got: {:?}",
                format_marker(&value.span.start),
                value
            )),
        }
    }
}

impl TryFrom<&saphyr::Yaml<'_>> for ConstValue {
    type Error = crate::Error;

    fn try_from(value: &saphyr::Yaml) -> Result<Self> {
        match value {
            saphyr::Yaml::Value(scalar) => scalar.try_into(),
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
                let enum_schema = EnumSchema::from_annotated_mapping(mapping)?;
                return Ok(Schema::Enum(enum_schema));
            } else if mapping.contains_key(&MarkedYaml::value_from_str("const")) {
                let const_schema = ConstSchema::from_annotated_mapping(mapping)?;
                return Ok(Schema::Const(const_schema));
            } else if mapping.contains_key(&MarkedYaml::value_from_str("anyOf")) {
                let any_of_schema = AnyOfSchema::from_annotated_mapping(mapping)?;
                return Ok(Schema::AnyOf(any_of_schema));
            } else if mapping.contains_key(&MarkedYaml::value_from_str("oneOf")) {
                let one_of_schema = marked_yaml.try_into()?;
                return Ok(Schema::OneOf(one_of_schema));
            } else if mapping.contains_key(&MarkedYaml::value_from_str("not")) {
                let not_schema = NotSchema::from_annotated_mapping(mapping)?;
                return Ok(Schema::Not(not_schema));
            } else {
                return Err(generic_error!(
                    "(Schema) Don't know how to construct schema: {:?}",
                    mapping
                ));
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

impl FromAnnotatedMapping<Schema> for Schema {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> Result<Self> {
        if mapping.is_empty() {
            Err(generic_error!("Empty mapping"))
        } else if mapping.contains_key(&MarkedYaml::value_from_str("type")) {
            match TypedSchema::from_annotated_mapping(mapping) {
                Ok(typed_schema) => Ok(typed_schema.into()),
                Err(e) => Err(e),
            }
        } else if mapping.contains_key(&MarkedYaml::value_from_str("enum")) {
            let enum_schema = EnumSchema::from_annotated_mapping(mapping)?;
            return Ok(Schema::Enum(enum_schema));
        } else if mapping.contains_key(&MarkedYaml::value_from_str("const")) {
            let const_schema = ConstSchema::from_annotated_mapping(mapping)?;
            return Ok(Schema::Const(const_schema));
        } else if mapping.contains_key(&MarkedYaml::value_from_str("anyOf")) {
            let any_of_schema = AnyOfSchema::from_annotated_mapping(mapping)?;
            return Ok(Schema::AnyOf(any_of_schema));
        } else if mapping.contains_key(&MarkedYaml::value_from_str("oneOf")) {
            let one_of_schema = OneOfSchema::from_annotated_mapping(mapping)?;
            return Ok(Schema::OneOf(one_of_schema));
        } else if mapping.contains_key(&MarkedYaml::value_from_str("not")) {
            let not_schema = NotSchema::from_annotated_mapping(mapping)?;
            return Ok(Schema::Not(not_schema));
        } else {
            return Err(generic_error!(
                "Don't know how to construct schema: {:#?}",
                mapping
            ));
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
    use ordered_float::OrderedFloat;

    #[test]
    fn test_const_equality() {
        let i1 = ConstValue::integer(42);
        let i2 = ConstValue::integer(42);
        assert_eq!(i1, i2);

        let s1 = ConstValue::string("NW");
        let s2 = ConstValue::string("NW");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_scalar_to_constvalue() -> Result<()> {
        let scalars = [
            Scalar::Null,
            Scalar::Boolean(true),
            Scalar::Boolean(false),
            Scalar::Integer(42),
            Scalar::Integer(-1),
            Scalar::FloatingPoint(OrderedFloat::from(3.14)),
            Scalar::String("foo".into()),
        ];

        let expected = [
            ConstValue::Null,
            ConstValue::Boolean(true),
            ConstValue::Boolean(false),
            ConstValue::Number(Number::Integer(42)),
            ConstValue::Number(Number::Integer(-1)),
            ConstValue::Number(Number::Float(3.14)),
            ConstValue::String("foo".to_string()),
        ];

        for (scalar, expected) in scalars.iter().zip(expected.iter()) {
            let actual: ConstValue = scalar.try_into()?;
            assert_eq!(*expected, actual);
        }

        Ok(())
    }
}
