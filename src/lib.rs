//! yaml-schema is a library for validating YAML data against a JSON Schema.

use hashlink::LinkedHashMap;
use jsonptr::Pointer;
use log::debug;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;
use url::Url;

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
pub use reference::RefUri;
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

/// A RootSchema represents the root document in a schema document, and includes additional
/// fields such as `$schema` that are not allowed in subschemas. It also provides a way to
/// resolve references to other schemas.
#[derive(Debug, PartialEq)]
pub struct RootSchema<'r> {
    pub meta_schema: Option<String>,
    pub schema: YamlSchema<'r>,
    /// Base URI for resolving relative `$ref` values (from file path, URL, or `$id`).
    pub base_uri: Option<Url>,
}

impl<'r> RootSchema<'r> {
    /// Create an empty RootSchema
    pub fn empty() -> Self {
        Self {
            meta_schema: None,
            schema: YamlSchema::Empty,
            base_uri: None,
        }
    }

    /// Create a new RootSchema with a given schema
    pub fn new(schema: YamlSchema<'r>) -> Self {
        Self {
            meta_schema: None,
            schema,
            base_uri: None,
        }
    }

    /// Resolve a JSON Pointer to an element in the schema.
    pub fn resolve(&self, pointer: &Pointer) -> Option<&YamlSchema<'_>> {
        let components = pointer.components().collect::<Vec<_>>();
        debug!("[RootSchema#resolve] components: {components:?}");
        components.first().and_then(|component| {
            debug!("[RootSchema#resolve] component: {component:?}");
            match component {
                jsonptr::Component::Root => {
                    let components = &components[1..];
                    components.first().and_then(|component| {
                        debug!("[RootSchema#resolve] component: {component:?}");
                        match component {
                            jsonptr::Component::Root => unimplemented!(),
                            jsonptr::Component::Token(token) => {
                                self.schema.resolve(Some(token), &components[1..])
                            }
                        }
                    })
                }
                jsonptr::Component::Token(token) => {
                    self.schema.resolve(Some(token), &components[1..])
                }
            }
        })
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for RootSchema<'r> {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml<'r>) -> Result<Self> {
        match &marked_yaml.data {
            YamlData::Value(scalar) => match scalar {
                Scalar::Boolean(r#bool) => Ok(Self {
                    meta_schema: None,
                    schema: YamlSchema::<'r>::BooleanLiteral(*r#bool),
                    base_uri: None,
                }),
                Scalar::Null => Ok(RootSchema {
                    meta_schema: None,
                    schema: YamlSchema::<'r>::Null,
                    base_uri: None,
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
                    base_uri: None,
                })
            }
            _ => Err(generic_error!(
                "[loader#load_from_doc] Don't know how to load: {:?}",
                marked_yaml
            )),
        }
    }
}

impl Validator for RootSchema<'_> {
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

    pub fn to_f64(self) -> f64 {
        match self {
            Number::Integer(i) => i as f64,
            Number::Float(f) => f,
        }
    }

    pub fn is_multiple_of(self, divisor: Number) -> bool {
        match (self, divisor) {
            (Number::Integer(a), Number::Integer(b)) => b != 0 && a % b == 0,
            _ => {
                let d = divisor.to_f64();
                d != 0.0 && self.to_f64() % d == 0.0
            }
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => a.partial_cmp(b),
            _ => self.to_f64().partial_cmp(&other.to_f64()),
        }
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

/// A ConstValue represents a constant value for the `const` keyword.
/// Per JSON Schema, `const` can be any JSON value: null, boolean, number,
/// string, array, or object.
#[derive(Debug, PartialEq)]
pub enum ConstValue {
    Null,
    Boolean(bool),
    Number(Number),
    String(String),
    Array(Vec<ConstValue>),
    Object(LinkedHashMap<String, ConstValue>),
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

    pub fn accepts(&self, value: &saphyr::MarkedYaml) -> bool {
        match self {
            ConstValue::Null => matches!(&value.data, YamlData::Value(Scalar::Null)),
            ConstValue::Boolean(expected) => {
                matches!(&value.data, YamlData::Value(Scalar::Boolean(actual)) if *expected == *actual)
            }
            ConstValue::Number(number) => match (number, &value.data) {
                (Number::Integer(expected), YamlData::Value(Scalar::Integer(actual))) => {
                    *actual == *expected
                }
                (Number::Float(expected), YamlData::Value(Scalar::FloatingPoint(of))) => {
                    of.into_inner() == *expected
                }
                _ => false,
            },
            ConstValue::String(expected) => {
                matches!(&value.data, YamlData::Value(Scalar::String(actual)) if expected == actual.as_ref())
            }
            ConstValue::Array(expected) => {
                if let YamlData::Sequence(actual) = &value.data {
                    expected.len() == actual.len()
                        && expected
                            .iter()
                            .zip(actual.iter())
                            .all(|(exp, act)| exp.accepts(act))
                } else {
                    false
                }
            }
            ConstValue::Object(expected) => {
                if let YamlData::Mapping(actual) = &value.data {
                    expected.len() == actual.len()
                        && expected.iter().all(|(key, exp_val)| {
                            let key_yaml = MarkedYaml::value_from_str(key);
                            actual
                                .get(&key_yaml)
                                .is_some_and(|act_yaml| exp_val.accepts(act_yaml))
                        })
                } else {
                    false
                }
            }
        }
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
            YamlData::Sequence(seq) => {
                let arr = seq
                    .iter()
                    .map(|item| item.try_into())
                    .collect::<Result<Vec<_>>>()?;
                Ok(ConstValue::Array(arr))
            }
            YamlData::Mapping(mapping) => {
                let mut obj = LinkedHashMap::new();
                for (key, val) in mapping.iter() {
                    let key_str = marked_yaml_to_string(key, "const object key must be a string")?;
                    let val_cv: ConstValue = val.try_into()?;
                    obj.insert(key_str, val_cv);
                }
                Ok(ConstValue::Object(obj))
            }
            YamlData::Tagged(_, inner) => (&inner.data).try_into(),
            YamlData::Representation(_, _, _) | YamlData::Alias(_) | YamlData::BadValue => Err(
                generic_error!("Unsupported YamlData variant for const: {:?}", value),
            ),
        }
    }
}

impl<'a> TryFrom<&MarkedYaml<'a>> for ConstValue {
    type Error = crate::Error;
    fn try_from(value: &MarkedYaml<'a>) -> Result<ConstValue> {
        (&value.data).try_into()
    }
}

impl std::fmt::Display for ConstValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstValue::Boolean(b) => write!(f, "{b} (bool)"),
            ConstValue::Null => write!(f, "null"),
            ConstValue::Number(n) => write!(f, "{n} (number)"),
            ConstValue::String(s) => write!(f, "\"{s}\""),
            ConstValue::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            ConstValue::Object(obj) => {
                write!(f, "{{")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{k}\": {v}")?;
                }
                write!(f, "}}")
            }
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
    use saphyr::LoadableYamlNode;

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

    #[test]
    fn test_const_value_array_try_from() -> Result<()> {
        let docs = MarkedYaml::load_from_str("[1, 2, 3]").unwrap();
        let cv: ConstValue = docs.first().unwrap().try_into()?;
        assert_eq!(
            cv,
            ConstValue::Array(vec![
                ConstValue::integer(1),
                ConstValue::integer(2),
                ConstValue::integer(3),
            ])
        );
        Ok(())
    }

    #[test]
    fn test_const_value_object_try_from() -> Result<()> {
        let docs = MarkedYaml::load_from_str("a: 1\nb: two").unwrap();
        let cv: ConstValue = docs.first().unwrap().try_into()?;
        let mut expected = LinkedHashMap::new();
        expected.insert("a".into(), ConstValue::integer(1));
        expected.insert("b".into(), ConstValue::string("two"));
        assert_eq!(cv, ConstValue::Object(expected));
        Ok(())
    }

    #[test]
    fn test_const_value_accepts_array() -> Result<()> {
        let cv = ConstValue::Array(vec![ConstValue::integer(1), ConstValue::string("foo")]);
        let matching = MarkedYaml::load_from_str("[1, \"foo\"]").unwrap();
        let not_matching = MarkedYaml::load_from_str("[1, \"bar\"]").unwrap();
        assert!(cv.accepts(matching.first().unwrap()));
        assert!(!cv.accepts(not_matching.first().unwrap()));
        Ok(())
    }

    #[test]
    fn test_const_value_accepts_object() -> Result<()> {
        let mut obj = LinkedHashMap::new();
        obj.insert("x".into(), ConstValue::integer(42));
        obj.insert("y".into(), ConstValue::string("hi"));
        let cv = ConstValue::Object(obj);
        let matching = MarkedYaml::load_from_str("x: 42\ny: hi").unwrap();
        let not_matching = MarkedYaml::load_from_str("x: 43\ny: hi").unwrap();
        assert!(cv.accepts(matching.first().unwrap()));
        assert!(!cv.accepts(not_matching.first().unwrap()));
        Ok(())
    }
}
