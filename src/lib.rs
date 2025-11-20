use std::rc::Rc;

use hashlink::LinkedHashMap;
use saphyr::LoadableYamlNode;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

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
pub use schemas::Schema;
pub use schemas::StringSchema;
pub use schemas::TypedSchema;
pub use schemas::YamlSchema;
pub use validation::Context;
pub use validation::Validator;

use crate::utils::format_marker;

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
/// we treat integers and floating point values as 'fungible' and represent them
/// using the `Number` enum.
#[derive(Debug, PartialEq)]
pub enum ConstValue {
    Null,
    Boolean(bool),
    Number(Number),
    String(String),
}

impl ConstValue {
    pub fn null() -> ConstValue {
        ConstValue::Null
    }
    pub fn boolean(value: bool) -> ConstValue {
        ConstValue::Boolean(value)
    }
    pub fn integer(value: i64) -> ConstValue {
        ConstValue::Number(Number::integer(value))
    }
    pub fn float(value: f64) -> ConstValue {
        ConstValue::Number(Number::float(value))
    }
    pub fn string<V: Into<String>>(value: V) -> ConstValue {
        ConstValue::String(value.into())
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
