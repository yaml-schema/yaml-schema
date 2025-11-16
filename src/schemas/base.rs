use std::collections::HashMap;

use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::ConstValue;
use crate::schemas::SchemaMetadata;
use crate::schemas::r#enum::load_enum_values;
use crate::utils::format_marker;

/// The `BaseSchema` contains common fields for all schemas.
#[derive(Debug, Default, PartialEq)]
pub struct BaseSchema {
    pub r#enum: Option<Vec<ConstValue>>,
    pub r#const: Option<ConstValue>,
    pub description: Option<String>,
}

impl BaseSchema {
    pub fn as_hash_map(&self) -> HashMap<String, String> {
        let mut h = HashMap::new();
        if let Some(r#enum) = &self.r#enum {
            h.insert(
                "enum".to_string(),
                r#enum
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            );
        }
        if let Some(r#const) = &self.r#const {
            h.insert("const".to_string(), r#const.to_string());
        }
        h
    }

    pub fn handle_key_value(
        &mut self,
        key: &str,
        value: &MarkedYaml,
    ) -> crate::Result<Option<&Self>> {
        match key {
            "enum" => {
                if let YamlData::Sequence(values) = &value.data {
                    self.r#enum = Some(load_enum_values(values)?);
                    Ok(Some(self))
                } else {
                    Err(expected_scalar!(
                        "{} Expected an array for enum:, but got: {:#?}",
                        format_marker(&value.span.start),
                        value
                    ))
                }
            }
            "const" => {
                if let YamlData::Value(scalar) = &value.data {
                    let const_value: ConstValue = scalar.try_into()?;
                    self.r#const = Some(const_value);
                    Ok(Some(self))
                } else {
                    Err(expected_scalar!(
                        "{} Expecting scalar value for const, got {:?}",
                        format_marker(&value.span.start),
                        value
                    ))
                }
            }
            "description" => {
                if let YamlData::Value(Scalar::String(value)) = &value.data {
                    self.description = Some(value.to_string());
                    Ok(Some(self))
                } else {
                    Err(expected_scalar!(
                        "{} Expected a string value for description, got {:?}",
                        format_marker(&value.span.start),
                        value
                    ))
                }
            }
            _ => Ok(None),
        }
    }
}

impl SchemaMetadata for BaseSchema {
    fn get_accepted_keys() -> &'static [&'static str] {
        &["type", "enum", "const", "description"]
    }
}

impl TryFrom<&MarkedYaml<'_>> for BaseSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml) -> Result<Self, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
            Ok(BaseSchema::try_from(mapping)?)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for BaseSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let mut base_schema = BaseSchema::default();
        for (key, value) in mapping.iter() {
            if let YamlData::Value(Scalar::String(key)) = &key.data {
                base_schema.handle_key_value(key, value)?;
            } else {
                return Err(generic_error!(
                    "{} Expected string key, got {:?}",
                    format_marker(&key.span.start),
                    key
                ));
            }
        }
        Ok(base_schema)
    }
}

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode as _;

    use super::*;

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
            base_schema.r#enum,
            Some(vec![
                ConstValue::String("foo".to_string()),
                ConstValue::String("bar".to_string())
            ])
        );
    }
}
