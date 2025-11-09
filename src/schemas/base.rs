use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::schemas::r#enum::load_enum_values;
use crate::utils::format_marker;
use crate::ConstValue;

/// A `SchemaTypeValue` is either a string or an array of strings
#[derive(Debug)]
pub enum SchemaTypeValue {
    Single(String),
    Multiple(Vec<String>),
}

/// The `BaseSchema` contains common fields for all schemas.
#[derive(Debug, Default)]
pub struct BaseSchema {
    pub r#type: Option<SchemaTypeValue>,
    pub r#enum: Option<Vec<ConstValue>>,
    pub r#const: Option<ConstValue>,
}

impl TryFrom<&MarkedYaml<'_>> for BaseSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<Self, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
            let mut base_schema = BaseSchema::default();
            for (key, value) in mapping.iter() {
                if let YamlData::Value(Scalar::String(key)) = &key.data {
                    match key.as_ref() {
                        "type" => {
                            unimplemented!();
                        }
                        "enum" => {
                            if let YamlData::Sequence(values) = &value.data {
                                base_schema.r#enum = Some(load_enum_values(values)?);
                            } else {
                                return Err(generic_error!(
                                    "{} Expected an array for enum:, but got: {:#?}",
                                    format_marker(&value.span.start),
                                    value
                                ));
                            }
                        }
                        "const" => {
                            if let YamlData::Value(scalar) = &value.data {
                                let const_value: ConstValue = scalar.try_into()?;
                                base_schema.r#const = Some(const_value);
                            } else {
                                return Err(generic_error!(
                                    "{} Expecting scalar value for const, got {:?}",
                                    format_marker(&value.span.start),
                                    value
                                ));
                            }
                        }
                        _ => (),
                    }
                } else {
                    return Err(generic_error!(
                        "{} Expected string key, got {:?}",
                        format_marker(&key.span.start),
                        key
                    ));
                }
            }
            Ok(base_schema)
        } else {
            Err(expected_mapping!(value))
        }
    }
}
