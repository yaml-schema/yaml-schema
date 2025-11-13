use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::ArraySchema;
use crate::IntegerSchema;
use crate::NumberSchema;
use crate::ObjectSchema;
use crate::Result;
use crate::StringSchema;
use crate::Validator;
use crate::loader::FromAnnotatedMapping;
use crate::utils::format_scalar;

/// A TypedSchema is a subset of YamlSchema that has a `type:`
#[derive(Debug, PartialEq)]
pub enum TypedSchema {
    /// `type: null`
    Null,
    /// `type: array`
    Array(ArraySchema),
    /// `type: boolean`
    BooleanSchema,
    /// `type: integer`
    Integer(IntegerSchema),
    /// `type: number`
    Number(NumberSchema),
    /// `type: object`
    Object(Box<ObjectSchema>),
    /// `type: string`
    String(StringSchema),
}

impl TryFrom<&MarkedYaml<'_>> for TypedSchema {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml<'_>) -> crate::Result<Self> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            let type_key = MarkedYaml::value_from_str("type");
            if mapping.contains_key(&type_key) {
                let value = mapping.get(&type_key).unwrap();
                match &value.data {
                    YamlData::Value(scalar) => match scalar {
                        Scalar::String(s) => {
                            try_typed_schema_from_mapping_with_type(s.as_ref(), marked_yaml)
                        }
                        saphyr::Scalar::Null => Ok(TypedSchema::Null),
                        v => Err(unsupported_type!(
                            "Expected a string value for 'type:', but got: {}",
                            format_scalar(v)
                        )),
                    },
                    YamlData::Sequence(values) => {
                        println!("values: {values:?}");
                        let type_values = values
                            .iter()
                            .map(|v| {
                                if let YamlData::Value(Scalar::String(s)) = &v.data {
                                    Ok(s.as_ref())
                                } else {
                                    Err(expected_scalar!(
                                        "Expected a string value for 'type:', but got: {:#?}",
                                        v
                                    ))
                                }
                            })
                            .collect::<Result<Vec<&str>>>()?
                            .iter()
                            .map(|r#type| {
                                try_typed_schema_from_mapping_with_type(r#type, marked_yaml)
                            })
                            .collect::<Result<Vec<TypedSchema>>>();
                        match type_values {
                            Ok(type_values) => {
                                println!("type_values: {type_values:?}");
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                        unimplemented!()
                    }
                    v => Err(expected_scalar!("Expected scalar type, but got: {:#?}", v)),
                }
            } else {
                Err(generic_error!(
                    "No type key found in mapping: {:#?}",
                    mapping
                ))
            }
        } else {
            Err(expected_mapping!(marked_yaml))
        }
    }
}

fn try_typed_schema_from_mapping_with_type(
    r#type: &str,
    marked_yaml: &MarkedYaml<'_>,
) -> Result<TypedSchema> {
    let mapping = marked_yaml
        .data
        .as_mapping()
        .expect("[try_typed_schema_from_mapping_with_type] Expected a mapping");
    match r#type {
        "array" => {
            let array_schema = ArraySchema::from_annotated_mapping(mapping)?;
            Ok(TypedSchema::Array(array_schema))
        }
        "boolean" => Ok(TypedSchema::BooleanSchema),
        "integer" => {
            let integer_schema: IntegerSchema = marked_yaml.try_into()?;
            Ok(TypedSchema::Integer(integer_schema))
        }
        "number" => {
            let number_schema: NumberSchema = marked_yaml.try_into()?;
            Ok(TypedSchema::Number(number_schema))
        }
        "object" => {
            let object_schema: ObjectSchema = marked_yaml.try_into()?;
            Ok(TypedSchema::Object(Box::new(object_schema)))
        }
        "string" => {
            let string_schema: StringSchema = marked_yaml.try_into()?;
            Ok(TypedSchema::String(string_schema))
        }
        s => Err(unsupported_type!(s.to_string())),
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for TypedSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let type_key = MarkedYaml::value_from_str("type");
        if mapping.contains_key(&type_key) {
            let value = mapping.get(&type_key).unwrap();
            match &value.data {
                YamlData::Value(scalar) => match scalar {
                    Scalar::String(s) => match s.as_ref() {
                        "array" => {
                            let array_schema = ArraySchema::from_annotated_mapping(mapping)?;
                            Ok(TypedSchema::Array(array_schema))
                        }
                        "boolean" => Ok(TypedSchema::BooleanSchema),
                        "integer" => {
                            let integer_schema: IntegerSchema = value.try_into()?;
                            Ok(TypedSchema::Integer(integer_schema))
                        }
                        "number" => {
                            let number_schema: NumberSchema = value.try_into()?;
                            Ok(TypedSchema::Number(number_schema))
                        }
                        "object" => {
                            let object_schema: ObjectSchema = value.try_into()?;
                            Ok(TypedSchema::Object(Box::new(object_schema)))
                        }
                        "string" => {
                            let string_schema: StringSchema = value.try_into()?;
                            Ok(TypedSchema::String(string_schema))
                        }
                        s => Err(unsupported_type!(s.to_string())),
                    },
                    saphyr::Scalar::Null => Ok(TypedSchema::Null),
                    v => Err(unsupported_type!(
                        "Expected a string value for 'type:', but got: {}",
                        format_scalar(v)
                    )),
                },
                v => Err(expected_scalar!("Expected scalar type, but got: {:#?}", v)),
            }
        } else {
            Err(generic_error!(
                "No type key found in mapping: {:#?}",
                mapping
            ))
        }
    }
}

impl std::fmt::Display for TypedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedSchema::Array(a) => write!(f, "{a}"),
            TypedSchema::BooleanSchema => write!(f, "type: boolean"),
            TypedSchema::Null => write!(f, "type: null"),
            TypedSchema::Integer(i) => write!(f, "{i}"),
            TypedSchema::Number(n) => write!(f, "{n}"),
            TypedSchema::Object(o) => write!(f, "{o}"),
            TypedSchema::String(s) => write!(f, "{s}"),
        }
    }
}

impl Validator for TypedSchema {
    fn validate(&self, context: &crate::Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[TypedSchema] self: {self}");
        debug!("[TypedSchema] Validating value: {value:?}");
        match self {
            TypedSchema::Array(a) => a.validate(context, value),
            TypedSchema::BooleanSchema => Ok(()),
            TypedSchema::Null => {
                if !value.data.is_null() {
                    context.add_error(value, format!("Expected null, but got: {value:?}"));
                }
                Ok(())
            }
            TypedSchema::Integer(i) => i.validate(context, value),
            TypedSchema::Number(n) => n.validate(context, value),
            TypedSchema::Object(o) => o.validate(context, value),
            TypedSchema::String(s) => s.validate(context, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode;

    use crate::{ConstValue, Context};

    use super::*;

    #[test]
    fn test_typed_schema_try_from_type_null() {
        let doc = MarkedYaml::load_from_str("type: null").unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        assert_eq!(typed_schema, TypedSchema::Null);
    }

    #[test]
    fn test_typed_schema_try_from_type_string() {
        let doc = MarkedYaml::load_from_str("type: string").unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        assert_eq!(typed_schema, TypedSchema::String(StringSchema::default()));
    }

    #[test]
    fn test_typed_schema_with_enum() {
        // Given a YAML schema with a string type and an enum
        let yaml = r#"
        type: string
        enum:
            - "foo"
            - "bar"
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        assert!(matches!(typed_schema, TypedSchema::String(_)));
        let TypedSchema::String(string_schema) = typed_schema else {
            panic!("Expected TypedSchema::String, but got: {typed_schema:?}");
        };
        assert_eq!(
            string_schema.base.r#enum,
            Some(vec![ConstValue::string("foo"), ConstValue::string("bar"),])
        );
        // When we validate a value that is in the enum
        let context = Context::default();
        string_schema
            .validate(&context, &MarkedYaml::value_from_str("foo"))
            .expect("validate() failed!");
        // Then we should not have any errors
        assert!(!context.has_errors());
        // When we validate a value that is not in the enum
        string_schema
            .validate(&context, &MarkedYaml::value_from_str("baz"))
            .expect("validate() failed!");
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        // Then we should have one error
        assert_eq!(errors.len(), 1);
        // And the error should be that the value is not in the enum
        assert_eq!(
            errors.first().unwrap().error,
            "String is not in enum: [\"foo\", \"bar\"]"
        );
    }

    #[ignore = "Not yet implemented"]
    #[test]
    fn test_multiple_types() {
        // Given a YAML schema with a string type and a number type
        let yaml = r#"
        type: [string, number]
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        println!("typed_schema: {typed_schema:?}");
        // When we validate a value that is a string
        let context = Context::default();
        typed_schema
            .validate(&context, &MarkedYaml::value_from_str("foo"))
            .expect("validate() failed!");
        // Then we should not have any errors
        assert!(!context.has_errors());
        // When we validate a value that is a number
        typed_schema
            .validate(&context, &MarkedYaml::value_from_str("42"))
            .expect("validate() failed!");
        // Then we should not have any errors
        assert!(!context.has_errors());
        // When we validate a value that is not a string or number
        typed_schema
            .validate(
                &context,
                &MarkedYaml::value_from_str("an: [arbitrarily, nested, data, structure]"),
            )
            .expect("validate() failed!");
        // Then we should have one error
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        assert_eq!(errors.len(), 1);
        // And the error should be that the value is not a string or number
        assert_eq!(
            errors.first().unwrap().error,
            "Expected a string or number, but got: `an: [arbitrarily, nested, data, structure]`"
        );
    }
}
