use std::collections::HashMap;
use std::fmt::Display;

use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::ArraySchema;
use crate::ConstValue;
use crate::Error;
use crate::IntegerSchema;
use crate::NumberSchema;
use crate::ObjectSchema;
use crate::Result;
use crate::StringSchema;
use crate::Validator;
use crate::schemas::BaseSchema;
use crate::schemas::SchemaMetadata as _;
use crate::utils::collect_keys;
use crate::utils::filter_mapping;
use crate::utils::format_hash_map;
use crate::utils::format_scalar;
use crate::utils::format_vec;
use crate::utils::format_yaml_data;

/// A TypedSchema is a subset of YamlSchema that has a `type:`
/// It can be a single type or an aggregate of types.
///
/// # Examples
///
/// ```yaml
/// type: string
/// ```
///
/// ```yaml
/// type: [string, number]
/// minimum: 0
/// minLength: 1
/// ```
#[derive(PartialEq)]
pub struct TypedSchema {
    pub r#type: Vec<TypedSchemaType>,
    pub r#enum: Option<Vec<ConstValue>>,
    pub r#const: Option<ConstValue>,
}

impl TypedSchema {
    pub fn single(r#type: TypedSchemaType) -> Self {
        Self {
            r#type: vec![r#type],
            r#enum: None,
            r#const: None,
        }
    }

    pub fn array(array_schema: ArraySchema) -> Self {
        Self::single(TypedSchemaType::Array(array_schema))
    }

    pub fn boolean() -> Self {
        Self::single(TypedSchemaType::BooleanSchema)
    }

    pub fn integer(integer_schema: IntegerSchema) -> Self {
        Self::single(TypedSchemaType::Integer(integer_schema))
    }

    pub fn number(number_schema: NumberSchema) -> Self {
        Self::single(TypedSchemaType::Number(number_schema))
    }

    pub fn object(object_schema: ObjectSchema) -> Self {
        Self::single(TypedSchemaType::Object(Box::new(object_schema)))
    }

    pub fn string(string_schema: StringSchema) -> Self {
        Self::single(TypedSchemaType::String(string_schema))
    }

    pub fn null() -> Self {
        Self {
            r#type: vec![TypedSchemaType::Null],
            r#enum: None,
            r#const: None,
        }
    }
}

impl TryFrom<&MarkedYaml<'_>> for TypedSchema {
    type Error = crate::Error;

    fn try_from(marked_yaml: &MarkedYaml<'_>) -> crate::Result<Self> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            Ok(TypedSchema::try_from(mapping)?)
        } else {
            Err(expected_mapping!(marked_yaml))
        }
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for TypedSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let type_key = MarkedYaml::value_from_str("type");
        if let Some(type_value) = mapping.get(&type_key) {
            match &type_value.data {
                // singly typed schema
                YamlData::Value(scalar) => match scalar {
                    Scalar::String(s) => Ok((s.as_ref(), mapping).try_into()?),
                    saphyr::Scalar::Null => Ok(TypedSchema::null()),
                    v => Err(schema_loading_error!(
                        "Expected a string value for `type:`, but got: {}",
                        format_scalar(v)
                    )),
                },
                // multiple typed schema
                YamlData::Sequence(values) => {
                    debug!("[TypedSchema] values: {values:?}");
                    // Check that type values are all strings, then convert to TypedSchemaType
                    // then return a TypedSchema with r#type set to the TypedSchemaType values
                    values
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
                        .into_iter()
                        .map(|r#type| (r#type, mapping).try_into())
                        .collect::<Result<Vec<TypedSchemaType>>>()
                        .map(|type_values| TypedSchema {
                            r#type: type_values,
                            r#enum: None,
                            r#const: None,
                        })
                }
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

impl TryFrom<(&str, &MarkedYaml<'_>)> for TypedSchema {
    type Error = crate::Error;

    fn try_from((r#type, marked_yaml): (&str, &MarkedYaml<'_>)) -> crate::Result<Self> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            Ok(TypedSchema::try_from((r#type, mapping))?)
        } else {
            Err(expected_mapping!(marked_yaml))
        }
    }
}

impl TryFrom<(&str, &AnnotatedMapping<'_, MarkedYaml<'_>>)> for TypedSchema {
    type Error = crate::Error;

    fn try_from(
        (r#type, mapping): (&str, &AnnotatedMapping<'_, MarkedYaml<'_>>),
    ) -> crate::Result<Self> {
        match r#type {
            "array" => Ok(TypedSchema::array(ArraySchema::try_from(mapping)?)),
            "boolean" => Ok(TypedSchema::boolean()),
            "integer" => Ok(TypedSchema::integer(IntegerSchema::try_from(mapping)?)),
            "null" => Ok(TypedSchema::null()),
            "number" => Ok(TypedSchema::number(NumberSchema::try_from(mapping)?)),
            "object" => Ok(TypedSchema::object(ObjectSchema::try_from(mapping)?)),
            "string" => Ok(TypedSchema::string(StringSchema::try_from(mapping)?)),
            s => Err(unsupported_type!(s.to_string())),
        }
    }
}

impl Validator for TypedSchema {
    fn validate(&self, context: &crate::Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[TypedSchema#validate] self: {self:#?}");
        debug!(
            "[TypedSchema#validate] Validating value: {}",
            format_yaml_data(&value.data)
        );
        debug!(
            "[TypedSchema#validate] context.fail_fast: {}",
            context.fail_fast
        );

        // To simplify the logic, if single type we validate the value against the one type
        if self.r#type.len() == 1 {
            let typed_schema_type = self.r#type.first().expect("Expeted single type");
            let sub_context = context.get_sub_context();
            match typed_schema_type.validate(&sub_context, value) {
                Ok(()) => {
                    debug!(
                        "[TypedSchema#validate] sub_context.has_errors(): {}",
                        sub_context.has_errors()
                    );
                    if sub_context.has_errors() {
                        context.extend_errors(sub_context.errors.take());
                    }
                    Ok(())
                }
                Err(Error::FailFast) => {
                    if sub_context.has_errors() {
                        context.extend_errors(sub_context.errors.take());
                    }
                    Err(Error::FailFast)
                }
                Err(e) => Err(e),
            }
        } else {
            // If multiple types, we validate the value against each of the types
            // If any of the types validate successfully, we return Ok
            // If all the types fail, we return the first error
            debug!(
                "[TypedSchema] Validating value: {:?} against multiple types",
                value.data
            );
            for typed_schema_type in self.r#type.iter() {
                debug!("[TypedSchema] Validating against type: {typed_schema_type}");
                let sub_context = context.get_sub_context();
                let sub_result = typed_schema_type.validate(&sub_context, value);
                debug!("[TypedSchema] sub_result: {sub_result:?}");
                debug!(
                    "[TypedSchema] sub_context.has_errors: {}",
                    sub_context.has_errors()
                );
                match sub_result {
                    Ok(()) | Err(Error::FailFast) => {
                        if sub_context.has_errors() {
                            continue;
                        }
                        return Ok(());
                    }
                    Err(e) => return Err(e),
                }
            }
            // If we get here, then all the types failed
            let types = self
                .r#type
                .iter()
                .map(|t| t.str_type())
                .collect::<Vec<&str>>()
                .join(", ");
            context.add_error(
                value,
                format!(
                    "Expected one of [{types}] but got: {}",
                    format_yaml_data(&value.data)
                ),
            );
            Ok(())
        }
    }
}

impl Display for TypedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypedSchema {{ r#type: {:?} }}", self.r#type)
    }
}

impl std::fmt::Debug for TypedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut h = HashMap::new();
        if self.r#type.len() == 1 {
            h.insert(
                "type".to_string(),
                match self.r#type.first() {
                    Some(t) => format!("{:?}", t),
                    None => "<null>".to_string(),
                },
            );
        } else {
            h.insert(
                "types".to_string(),
                format_vec(
                    &self
                        .r#type
                        .iter()
                        .map(|t| format!("{:?}", t))
                        .collect::<Vec<String>>(),
                ),
            );
        }
        if let Some(r#enum) = &self.r#enum {
            h.insert("enum".to_string(), format_vec(r#enum));
        }
        if let Some(r#const) = &self.r#const {
            h.insert("const".to_string(), r#const.to_string());
        }
        write!(f, "TypedSchema {}", format_hash_map(&h))
    }
}

#[derive(PartialEq)]
pub enum TypedSchemaType {
    Null,
    Array(ArraySchema),
    BooleanSchema,
    Integer(IntegerSchema),
    Number(NumberSchema),
    Object(Box<ObjectSchema>), // we box the ObjectSchema to avoid large size differences enum variants
    String(StringSchema),
}

impl TypedSchemaType {
    pub fn str_type(&self) -> &str {
        match self {
            TypedSchemaType::Array(_) => "array",
            TypedSchemaType::BooleanSchema => "boolean",
            TypedSchemaType::Null => "null",
            TypedSchemaType::Integer(_) => "integer",
            TypedSchemaType::Number(_) => "number",
            TypedSchemaType::Object(_) => "object",
            TypedSchemaType::String(_) => "string",
        }
    }
}

impl TryFrom<(&str, &MarkedYaml<'_>)> for TypedSchemaType {
    type Error = crate::Error;
    fn try_from((r#type, marked_yaml): (&str, &MarkedYaml<'_>)) -> crate::Result<Self> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            Ok(TypedSchemaType::try_from((r#type, mapping))?)
        } else {
            Err(expected_mapping!(marked_yaml))
        }
    }
}

impl TryFrom<(&str, &AnnotatedMapping<'_, MarkedYaml<'_>>)> for TypedSchemaType {
    type Error = crate::Error;

    fn try_from(
        (r#type, mapping): (&str, &AnnotatedMapping<'_, MarkedYaml<'_>>),
    ) -> crate::Result<Self> {
        Ok(match r#type {
            "array" => {
                let keys = collect_keys(
                    BaseSchema::get_accepted_keys(),
                    ArraySchema::get_accepted_keys(),
                );
                debug!("[TypedSchemaType] keys: {keys:?}");
                let filtered_mapping = filter_mapping(mapping, keys, r#type)?;
                debug!("[TypedSchemaType] filtered_mapping: {filtered_mapping:?}");
                TypedSchemaType::Array(ArraySchema::try_from(&filtered_mapping)?)
            }
            "boolean" => TypedSchemaType::BooleanSchema,
            "integer" => {
                let keys = collect_keys(
                    BaseSchema::get_accepted_keys(),
                    IntegerSchema::get_accepted_keys(),
                );
                debug!("[TypedSchemaType] keys: {keys:?}");
                let filtered_mapping = filter_mapping(mapping, keys, r#type)?;
                TypedSchemaType::Integer(IntegerSchema::try_from(&filtered_mapping)?)
            }
            "null" => TypedSchemaType::Null,
            "number" => {
                let keys = collect_keys(
                    BaseSchema::get_accepted_keys(),
                    NumberSchema::get_accepted_keys(),
                );
                debug!("[TypedSchemaType] keys: {keys:?}");
                let filtered_mapping = filter_mapping(mapping, keys, r#type)?;
                debug!("[TypedSchemaType] filtered_mapping: {filtered_mapping:?}");
                TypedSchemaType::Number(NumberSchema::try_from(&filtered_mapping)?)
            }
            "object" => {
                let keys = collect_keys(
                    BaseSchema::get_accepted_keys(),
                    ObjectSchema::get_accepted_keys(),
                );
                debug!("[TypedSchemaType] keys: {keys:?}");
                let filtered_mapping = filter_mapping(mapping, keys, r#type)?;
                TypedSchemaType::Object(Box::new(ObjectSchema::try_from(&filtered_mapping)?))
            }
            "string" => {
                let keys = collect_keys(
                    BaseSchema::get_accepted_keys(),
                    StringSchema::get_accepted_keys(),
                );
                debug!("[TypedSchemaType] keys: {keys:?}");
                let filtered_mapping = filter_mapping(mapping, keys, r#type)?;
                debug!("[TypedSchemaType] filtered_mapping: {filtered_mapping:?}");
                TypedSchemaType::String(StringSchema::try_from(&filtered_mapping)?)
            }
            s => return Err(unsupported_type!(s.to_string())),
        })
    }
}

impl std::fmt::Display for TypedSchemaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::fmt::Debug for TypedSchemaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedSchemaType::Array(a) => write!(f, "{a:?}"),
            TypedSchemaType::BooleanSchema => write!(f, "{{ type: boolean }}"),
            TypedSchemaType::Null => write!(f, "{{ type: null }}"),
            TypedSchemaType::Integer(i) => write!(f, "{i:?}"),
            TypedSchemaType::Number(n) => write!(f, "{n:?}"),
            TypedSchemaType::Object(o) => write!(f, "{o:?}"),
            TypedSchemaType::String(s) => write!(f, "{s:?}"),
        }
    }
}

impl Validator for TypedSchemaType {
    fn validate(&self, context: &crate::Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[TypedSchemaType] self: {self}");
        debug!("[TypedSchemaType] Validating value: {value:?}");
        match self {
            TypedSchemaType::Array(a) => a.validate(context, value),
            TypedSchemaType::BooleanSchema => {
                if !value.data.is_boolean() {
                    context.add_error(
                        value,
                        format!("Expected: boolean, but got: {:?}", value.data),
                    );
                }
                Ok(())
            }
            TypedSchemaType::Null => {
                debug!("[TypedSchemaType] Validating value is `null`: {value:?}");
                if !value.data.is_null() {
                    context.add_error(value, format!("Expected null, but got: {:?}", value.data));
                }
                Ok(())
            }
            TypedSchemaType::Integer(i) => i.validate(context, value),
            TypedSchemaType::Number(n) => n.validate(context, value),
            TypedSchemaType::Object(o) => o.validate(context, value),
            TypedSchemaType::String(s) => s.validate(context, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode;

    use crate::ConstValue;
    use crate::validation;

    use super::*;

    #[test]
    fn test_typed_schema_try_from_type_null() {
        let doc = MarkedYaml::load_from_str("type: null").unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        assert_eq!(typed_schema, TypedSchema::null());
    }

    #[test]
    fn test_typed_schema_try_from_type_string() {
        let doc = MarkedYaml::load_from_str("type: string").unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        assert_eq!(typed_schema, TypedSchema::string(StringSchema::default()));
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
        let marked_yaml = doc.first().unwrap();
        let typed_schema: TypedSchemaType =
            TypedSchemaType::try_from(("string", marked_yaml)).unwrap();
        assert!(matches!(typed_schema, TypedSchemaType::String(_)));
        let TypedSchemaType::String(string_schema) = typed_schema else {
            panic!("Expected TypedSchemaType::String, but got: {typed_schema:?}");
        };
        assert_eq!(
            string_schema.base.r#enum,
            Some(vec![ConstValue::string("foo"), ConstValue::string("bar"),])
        );
        // When we validate a value that is in the enum
        let context = validation::Context::default();
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

    #[test]
    fn test_multiple_types_ok() {
        // Given a YAML schema with a string type and a number type
        let yaml = r#"
        type: [string, number]
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        println!("typed_schema: {typed_schema:?}");
        let context = validation::Context::default();

        // When we validate a value that is a string
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
    }

    #[test]
    fn test_multiple_types_should_error() {
        // Given a YAML schema with a string type and a number type
        let yaml = r#"
        type: [string, number]
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let mapping = doc.first().unwrap();
        let typed_schema: TypedSchema = mapping.try_into().unwrap();
        println!("typed_schema: {typed_schema:?}");
        let context = validation::Context::default();

        // When we validate a value that is not a string or number
        let yaml = "an: [arbitrarily, nested, data, structure]";
        let doc = saphyr::MarkedYaml::load_from_str(yaml).unwrap();
        let marked_yaml = doc.first().unwrap();
        typed_schema
            .validate(&context, marked_yaml)
            .expect("validate() failed!");
        // Then we should have one error
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        assert_eq!(errors.len(), 1);
        // And the error should be that the value is not a string or number
        assert_eq!(
            errors.first().unwrap().error,
            "Expected one of [string, number] but got: [\"an\": [\"arbitrarily\", \"nested\", \"data\", \"structure\"]]"
        );
    }

    #[test]
    fn test_multiple_types_with_constraints() {
        // Given a YAML schema with a string type and a number type
        let yaml = r#"
        type:
          - string
          - number
        minimum: 1
        minLength: 1
        "#;
        let doc = MarkedYaml::load_from_str(yaml).unwrap();
        let marked_yaml = doc.first().unwrap();
        let typed_schema: TypedSchema = marked_yaml.try_into().unwrap();
        println!("typed_schema: {typed_schema:?}");
        let context = validation::Context::default();

        // When we validate `1`
        typed_schema
            .validate(&context, &MarkedYaml::value_from_str("1"))
            .expect("validate() failed!");
        // Then we should not have any errors
        assert!(!context.has_errors());

        // When we validate `"one"`
        typed_schema
            .validate(&context, &MarkedYaml::value_from_str("one"))
            .expect("validate() failed!");
        // Then we should not have any errors
        assert!(!context.has_errors());

        // But when we validate `0`
        typed_schema
            .validate(&context, &MarkedYaml::value_from_str("0"))
            .expect("validate() failed!");
        // Then we should have one error
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        assert_eq!(errors.len(), 1);
        // And the error should be that the value is not a string or number
        println!("errors: {errors:?}");
    }
}
