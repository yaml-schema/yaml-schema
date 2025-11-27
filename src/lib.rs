//! yaml-schema is a library for validating YAML data against a JSON Schema.

use log::debug;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

#[macro_use]
pub mod error;
pub mod engine;
pub mod loader;
pub mod reference;
pub mod schemas;
pub mod utils;
pub mod validation;

pub use engine::Engine;
pub use error::Error;
pub use reference::Reference;
pub use schemas::YamlSchema;
pub use validation::Context;
pub use validation::Validator;

use utils::format_marker;

use crate::loader::marked_yaml_to_string;

// Returns the library version, which reflects the crate version
pub fn version() -> String {
    clap::crate_version!().to_string()
}

// Alias for std::result::Result<T, yaml_schema::Error>
pub type Result<T> = std::result::Result<T, Error>;

/// Because we can't `impl TryFrom<T> for String`, we use an internal trait to allow for
/// `Into<String>` conversions that can fail.
trait TryToString {
    type Error;

    fn try_into(&self) -> Result<String>;
}

/// A RootSchema represents the root document in a schema document, and includes additional
/// fields such as `$schema` that are not allowed in subschemas.
#[derive(Debug, PartialEq)]
pub struct RootSchema {
    pub meta_schema: Option<String>,
    pub schema: YamlSchema,
}

impl RootSchema {
    /// Create an empty RootSchema
    pub fn empty() -> Self {
        Self {
            meta_schema: None,
            schema: YamlSchema::Empty,
        }
    }

    /// Create a new RootSchema with a given schema
    pub fn new(schema: YamlSchema) -> Self {
        Self {
            meta_schema: None,
            schema,
        }
    }

    pub fn get_def(&self, _name: &str) -> Option<&YamlSchema> {
        unimplemented!()
    }
}

impl TryFrom<&MarkedYaml<'_>> for RootSchema {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml<'_>) -> Result<Self> {
        match &marked_yaml.data {
            YamlData::Value(scalar) => match scalar {
                Scalar::Boolean(r#bool) => Ok(Self {
                    meta_schema: None,
                    schema: YamlSchema::BooleanLiteral(*r#bool),
                }),
                Scalar::Null => Ok(RootSchema {
                    meta_schema: None,
                    schema: YamlSchema::Null,
                }),
                _ => Err(generic_error!(
                    "[loader#load_from_doc] Don't know how to a handle scalar: {:?}",
                    scalar
                )),
            },
            YamlData::Mapping(mapping) => {
                debug!(
                    "[loader#load_from_doc] Found mapping, trying to load as RootSchema: {mapping:?}"
                );
                let meta_schema = mapping
                    .get(&MarkedYaml::value_from_str("$schema"))
                    .map(|my| marked_yaml_to_string(my, "$schema must be a string"))
                    .transpose()?;

                let schema = YamlSchema::try_from(marked_yaml)?;
                Ok(RootSchema {
                    meta_schema,
                    schema,
                })
            }
            _ => Err(generic_error!(
                "[loader#load_from_doc] Don't know how to load: {:?}",
                marked_yaml
            )),
        }
    }
}

impl Validator for RootSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        self.schema.validate(context, value)
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
    #[allow(clippy::approx_constant)]
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
