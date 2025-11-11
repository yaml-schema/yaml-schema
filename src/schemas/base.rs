use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::ConstValue;
use crate::schemas::r#enum::load_enum_values;
use crate::utils::format_marker;

/// A `SchemaTypeValue` is either a string or an array of strings
#[derive(Debug, PartialEq)]
pub enum SchemaTypeValue {
    Single(String),
    Multiple(Vec<String>),
}

/// The `BaseSchema` contains common fields for all schemas.
#[derive(Debug, Default, PartialEq)]
pub struct BaseSchema {
    pub r#type: Option<SchemaTypeValue>,
    pub r#enum: Option<Vec<ConstValue>>,
    pub r#const: Option<ConstValue>,
}

impl BaseSchema {
    pub fn type_integer() -> Self {
        Self {
            r#type: Some(SchemaTypeValue::Single("integer".to_string())),
            ..Default::default()
        }
    }

    pub fn type_number() -> Self {
        Self {
            r#type: Some(SchemaTypeValue::Single("number".to_string())),
            ..Default::default()
        }
    }
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
                            if let YamlData::Value(Scalar::String(value)) = &value.data {
                                base_schema.r#type =
                                    Some(SchemaTypeValue::Single(value.to_string()));
                            } else if let YamlData::Sequence(values) = &value.data {
                                let values = values
                                    .iter()
                                    .map(|v| {
                                        if let YamlData::Value(Scalar::String(value)) = &v.data {
                                            Ok(value.to_string())
                                        } else {
                                            Err(generic_error!(
                                                "{} Expected a string value for type, got {:?}",
                                                format_marker(&v.span.start),
                                                v
                                            ))
                                        }
                                    })
                                    .collect::<Result<Vec<String>, crate::Error>>()?;
                                base_schema.r#type = Some(SchemaTypeValue::Multiple(values));
                            } else {
                                return Err(generic_error!(
                                    "{} Expected string or array for type, got {:?}",
                                    format_marker(&value.span.start),
                                    value
                                ));
                            }
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

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode as _;

    use super::*;

    #[test]
    fn test_single_type_with_enum() {
        let yaml = r#"
        type: string
        enum:
            - "foo"
            - "bar"
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let marked_yaml = doc.first().unwrap();
        let base_schema = BaseSchema::try_from(marked_yaml).unwrap();
        assert_eq!(
            base_schema.r#type,
            Some(SchemaTypeValue::Single("string".to_string()))
        );
        assert_eq!(
            base_schema.r#enum,
            Some(vec![
                ConstValue::String("foo".to_string()),
                ConstValue::String("bar".to_string())
            ])
        );
        assert_eq!(base_schema.r#const, None);
    }

    #[test]
    fn test_multiple_types() {
        let yaml = r#"
        type:
            - string
            - number
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let marked_yaml = doc.first().unwrap();
        let base_schema = BaseSchema::try_from(marked_yaml).unwrap();
        assert_eq!(
            base_schema.r#type,
            Some(SchemaTypeValue::Multiple(vec![
                "string".to_string(),
                "number".to_string()
            ]))
        );
        assert_eq!(base_schema.r#enum, None);
        assert_eq!(base_schema.r#const, None);
    }

    #[test]
    fn test_base_schema_with_enum() {
        let yaml = r#"
        type: string
        enum:
            - "foo"
            - "bar"
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let marked_yaml = doc.first().unwrap();
        let base_schema = BaseSchema::try_from(marked_yaml).unwrap();
        assert_eq!(
            base_schema.r#type,
            Some(SchemaTypeValue::Single("string".to_string()))
        );
        assert_eq!(
            base_schema.r#enum,
            Some(vec![
                ConstValue::String("foo".to_string()),
                ConstValue::String("bar".to_string())
            ])
        );
    }
}
