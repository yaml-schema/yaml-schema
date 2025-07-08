use super::Validator;
use crate::loader::{FromAnnotatedMapping, FromSaphyrMapping};
use crate::utils::{format_marker, saphyr_yaml_string};
use crate::ConstValue;
use crate::Context;
use crate::Number;
use crate::Result;
use log::debug;
use saphyr::{AnnotatedMapping, MarkedYaml};

/// A const schema represents a constant value
#[derive(Debug, PartialEq)]
pub struct ConstSchema {
    pub r#const: ConstValue,
}

impl std::fmt::Display for ConstSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Const {:?}", self.r#const)
    }
}

impl FromSaphyrMapping<ConstSchema> for ConstSchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> Result<ConstSchema> {
        let value = mapping.get(&saphyr_yaml_string("const")).unwrap();
        match value {
            saphyr::Yaml::Value(scalar) => match scalar {
                saphyr::Scalar::String(s) => Ok(ConstSchema {
                    r#const: ConstValue::string(s.to_string()),
                }),
                saphyr::Scalar::Integer(i) => Ok(ConstSchema {
                    r#const: ConstValue::integer(*i),
                }),
                saphyr::Scalar::FloatingPoint(o) => {
                    let f = o.into_inner();
                    Ok(ConstSchema {
                        r#const: ConstValue::float(f),
                    })
                }
                _ => Err(generic_error!("Unsupported const value: {:#?}", value)),
            },
            _ => Err(expected_scalar!(
                "Expected a scalar value for const, but got: {:#?}",
                value
            )),
        }
    }
}

impl FromAnnotatedMapping<ConstSchema> for ConstSchema {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> Result<ConstSchema> {
        let value = mapping
            .get(&MarkedYaml::value_from_str("const"))
            .ok_or(generic_error!("No `const` key found"))?;
        match &value.data {
            saphyr::YamlData::Value(scalar) => match scalar {
                saphyr::Scalar::String(s) => Ok(ConstSchema {
                    r#const: ConstValue::string(s.to_string()),
                }),
                saphyr::Scalar::Integer(i) => Ok(ConstSchema {
                    r#const: ConstValue::integer(*i),
                }),
                saphyr::Scalar::FloatingPoint(o) => {
                    let f = o.into_inner();
                    Ok(ConstSchema {
                        r#const: ConstValue::float(f),
                    })
                }
                _ => Err(generic_error!(
                    "{} Unsupported const value: {:#?}",
                    format_marker(&value.span.start),
                    value
                )),
            },
            _ => Err(expected_scalar!(
                "{} Expected a scalar value for const, but got: {:#?}",
                format_marker(&value.span.start),
                value
            )),
        }
    }
}

impl Validator for ConstSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let data = &value.data;
        debug!(
            "Validating value: {:?} against const: {:?}",
            &data, self.r#const
        );
        if let saphyr::YamlData::Value(scalar) = data {
            let expected_value = &self.r#const;
            match expected_value {
                ConstValue::Boolean(b) => {
                    if data.as_bool() != Some(*b) {
                        let error =
                            format!("Const validation failed, expected: {b:?}, got: {data:?}");
                        context.add_error(value, error);
                    }
                }
                ConstValue::Null => {
                    if !data.is_null() {
                        let error =
                            format!("Const validation failed, expected: null, got: {data:?}");
                        context.add_error(value, error);
                    }
                }
                ConstValue::Number(n) => {
                    match n {
                        Number::Integer(i) => {
                            if let saphyr::Scalar::Integer(x) = scalar {
                                if x != i {
                                    let error =
                                        format!("Const validation failed, expected: {i}, got: {x}");
                                    context.add_error(value, error);
                                }
                            } else {
                                let error =
                                format!("Const validation failed, expected integer value, got: {data:?}");
                                context.add_error(value, error);
                            }
                        }
                        Number::Float(f) => {
                            if let saphyr::Scalar::FloatingPoint(o) = scalar {
                                if o.into_inner() != *f {
                                    let error = format!(
                                        "Const validation failed, expected: {f:?}, got: {data:?}"
                                    );
                                    context.add_error(value, error);
                                }
                            } else {
                                let error =
                                format!("Const validation failed, expecte floating point, got: {data:?}");
                                context.add_error(value, error);
                            }
                        }
                    }
                }
                ConstValue::String(s) => {
                    if data.as_str() != Some(s) {
                        let error =
                            format!("Const validation failed, expected: {s:?}, got: {data:?}");
                        context.add_error(value, error);
                    }
                }
            }
        } else {
            let error = format!("Const validation failed, expected scalar, got: {data:?}");
            context.add_error(value, error);
        }
        Ok(())
    }
}
