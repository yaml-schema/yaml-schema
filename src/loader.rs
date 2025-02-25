use hashlink::LinkedHashMap;

use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use crate::AnyOfSchema;
use crate::ArraySchema;
use crate::BoolOrTypedSchema;
use crate::ConstSchema;
use crate::ConstValue;
use crate::EnumSchema;
use crate::Error;
use crate::IntegerSchema;
use crate::NotSchema;
use crate::Number;
use crate::NumberSchema;
use crate::ObjectSchema;
use crate::OneOfSchema;
use crate::Reference;
use crate::Result;
use crate::RootSchema;
use crate::Schema;
use crate::StringSchema;
use crate::TypedSchema;
use crate::YamlSchema;

pub fn load_file<S: Into<String>>(path: S) -> Result<RootSchema> {
    let path_s = path.into();
    let fs_metadata = fs::metadata(&path_s)?;
    if !fs_metadata.is_file() {
        return Err(Error::FileNotFound(path_s.clone()));
    }
    let s = fs::read_to_string(&path_s)?;
    let docs = saphyr::Yaml::load_from_str(&s)?;
    if docs.is_empty() {
        return Ok(RootSchema::new(YamlSchema::empty())); // empty schema
    }
    load_from_doc(docs.first().unwrap())
}

pub fn load_from_doc(doc: &saphyr::Yaml) -> Result<RootSchema> {
    let mut loader = RootLoader::new();
    match doc {
        saphyr::Yaml::Boolean(r#bool) => {
            loader.set_schema(YamlSchema::boolean_literal(*r#bool));
        }
        saphyr::Yaml::Hash(hash) => {
            loader.load_root_schema(hash)?;
        }
        saphyr::Yaml::Null => {
            loader.set_schema(YamlSchema::null());
        }
        saphyr::Yaml::String(s) => match s.as_str() {
            "true" => {
                loader.set_schema(YamlSchema::boolean_literal(true));
            }
            "false" => {
                loader.set_schema(YamlSchema::boolean_literal(false));
            }
            s => return Err(generic_error!("Expected true or false, but got: {}", s)),
        },
        _ => {
            unimplemented!()
        }
    }
    Ok(loader.into()) // See From<Loader> for RootSchema below
}

#[derive(Debug, Default)]
struct RootLoader {
    pub id: Option<String>,
    pub meta_schema: Option<String>,
    pub title: Option<String>,
    pub defs: Option<LinkedHashMap<String, YamlSchema>>,
    pub description: Option<String>,
    pub schema: Option<YamlSchema>,
}

impl RootLoader {
    fn new() -> Self {
        RootLoader::default()
    }

    /// Set the loader schema
    /// Just a convenience function to avoid having to write
    /// `self.schema = Some(schema);`
    fn set_schema(&mut self, schema: YamlSchema) {
        self.schema = Some(schema);
    }

    fn load_root_schema(&mut self, hash: &saphyr::Hash) -> Result<()> {
        let mut data = LinkedHashMap::new();

        for (key, value) in hash.iter() {
            match key {
                saphyr::Yaml::String(s) => match s.as_str() {
                    "$id" => self.id = Some(yaml_to_string(value, "$id must be a string")?),
                    "$schema" => {
                        self.meta_schema = Some(yaml_to_string(value, "$schema must be a string")?)
                    }
                    "title" => self.title = Some(yaml_to_string(value, "title must be a string")?),
                    "description" => {
                        self.description =
                            Some(yaml_to_string(value, "description must be a string")?)
                    }
                    "$defs" | "definitions" => {
                        let hash = value.as_hash().ok_or_else(|| {
                            unsupported_type!("Expected a hash, but got: {:#?}", value)
                        })?;
                        let mut defs = LinkedHashMap::new();
                        for (key, value) in hash.iter() {
                            let schema = YamlSchema::construct(value.as_hash().unwrap())?;
                            let key_string = yaml_to_string(key, "key must be a string")?;
                            defs.insert(key_string, schema);
                        }
                        self.defs = Some(defs);
                    }
                    _ => {
                        data.insert(key.clone(), value.clone());
                    }
                },
                _ => {
                    data.insert(key.clone(), value.clone());
                }
            }
        }
        self.schema = Some(YamlSchema::construct(&data)?);
        Ok(())
    }
}

/// Convert a Loader to a RootSchema
/// Just sets the schema to a YamlSchema::Empty if the loader schema is None
impl From<RootLoader> for RootSchema {
    fn from(loader: RootLoader) -> Self {
        RootSchema {
            id: loader.id,
            meta_schema: loader.meta_schema,
            defs: loader.defs,
            schema: Rc::new(loader.schema.unwrap_or(YamlSchema::empty())),
        }
    }
}

impl Constructor<Option<Schema>> for Schema {
    fn construct(hash: &saphyr::Hash) -> Result<Option<Schema>> {
        if hash.is_empty() {
            Ok(None)
        } else if hash.contains_key(&sys("type")) {
            match TypedSchema::construct(hash) {
                Ok(typed_schema) => Ok(Some(typed_schema.into())),
                Err(e) => Err(e),
            }
        } else if hash.contains_key(&sys("enum")) {
            let enum_schema = EnumSchema::construct(hash)?;
            return Ok(Some(Schema::Enum(enum_schema)));
        } else if hash.contains_key(&sys("const")) {
            let const_schema = ConstSchema::construct(hash)?;
            return Ok(Some(Schema::Const(const_schema)));
        } else if hash.contains_key(&sys("anyOf")) {
            let any_of_schema = AnyOfSchema::construct(hash)?;
            return Ok(Some(Schema::AnyOf(any_of_schema)));
        } else if hash.contains_key(&sys("oneOf")) {
            let one_of_schema = OneOfSchema::construct(hash)?;
            return Ok(Some(Schema::OneOf(one_of_schema)));
        } else if hash.contains_key(&sys("not")) {
            let not_schema = NotSchema::construct(hash)?;
            return Ok(Some(Schema::Not(not_schema)));
        } else {
            return Err(generic_error!(
                "Don't know how to construct schema: {:#?}",
                hash
            ));
        }
    }
}

impl Constructor<YamlSchema> for YamlSchema {
    fn construct(hash: &saphyr::Hash) -> Result<YamlSchema> {
        let mut metadata = LinkedHashMap::new();
        let mut r#ref: Option<Reference> = None;
        let mut data = LinkedHashMap::new();

        for (key, value) in hash.iter() {
            match key {
                saphyr::Yaml::String(s) => match s.as_str() {
                    "$id" => {
                        metadata.insert(s.clone(), yaml_to_string(value, "$id must be a string")?);
                    }
                    "$schema" => {
                        metadata.insert(
                            s.clone(),
                            yaml_to_string(value, "$schema must be a string")?,
                        );
                    }
                    "$ref" => {
                        r#ref = Some(Reference::construct(hash)?);
                    }
                    "title" => {
                        metadata
                            .insert(s.clone(), yaml_to_string(value, "title must be a string")?);
                    }
                    "description" => {
                        metadata.insert(
                            s.clone(),
                            yaml_to_string(value, "description must be a string")?,
                        );
                    }
                    _ => {
                        data.insert(key.clone(), value.clone());
                    }
                },
                _ => {
                    data.insert(key.clone(), value.clone());
                }
            }
        }
        let schema = Schema::construct(&data)?;
        Ok(YamlSchema {
            metadata: if metadata.is_empty() {
                None
            } else {
                Some(metadata)
            },
            schema,
            r#ref,
        })
    }
}

impl Constructor<TypedSchema> for TypedSchema {
    fn construct(hash: &saphyr::Hash) -> Result<TypedSchema> {
        let type_key = saphyr::Yaml::String(String::from("type"));
        if hash.contains_key(&type_key) {
            let value = hash.get(&type_key).unwrap();
            match value {
                saphyr::Yaml::String(s) => match s.as_str() {
                    "array" => {
                        let array_schema = ArraySchema::construct(hash)?;
                        Ok(TypedSchema::Array(array_schema))
                    }
                    "boolean" => Ok(TypedSchema::BooleanSchema),
                    "integer" => {
                        let integer_schema = IntegerSchema::construct(hash)?;
                        Ok(TypedSchema::Integer(integer_schema))
                    }
                    "number" => {
                        let number_schema = NumberSchema::construct(hash)?;
                        Ok(TypedSchema::Number(number_schema))
                    }
                    "object" => {
                        let object_schema = ObjectSchema::construct(hash)?;
                        Ok(TypedSchema::Object(object_schema))
                    }
                    "string" => {
                        let string_schema = StringSchema::construct(hash)?;
                        Ok(TypedSchema::String(string_schema))
                    }
                    s => Err(unsupported_type!(s.to_string())),
                },
                saphyr::Yaml::Null => Ok(TypedSchema::Null),
                v => Err(unsupported_type!(
                    "Expected type: string, but got: {:#?}",
                    v
                )),
            }
        } else {
            Err(generic_error!("No type key found in hash: {:#?}", hash))
        }
    }
}

/// A Constructor constructs an object (a schema) from a saphyr::Hash
pub trait Constructor<T> {
    fn construct(hash: &saphyr::Hash) -> Result<T>;
}

fn load_string_value(value: &saphyr::Yaml) -> Result<String> {
    match value {
        saphyr::Yaml::String(s) => Ok(s.clone()),
        _ => Err(unsupported_type!(
            "Expected a string value, but got: {:?}",
            value
        )),
    }
}

impl Constructor<ArraySchema> for ArraySchema {
    fn construct(hash: &saphyr::Hash) -> Result<ArraySchema> {
        let mut array_schema = ArraySchema::default();
        for (key, value) in hash.iter() {
            if let saphyr::Yaml::String(key) = key {
                match key.as_str() {
                    "contains" => {
                        let yaml_schema = YamlSchema::construct(value.as_hash().unwrap())?;
                        array_schema.contains = Some(Box::new(yaml_schema));
                    }
                    "items" => {
                        let array_items = load_array_items(value)?;
                        array_schema.items = Some(array_items);
                    }
                    "type" => {
                        let s = load_string_value(value)?;
                        if s != "array" {
                            return Err(unsupported_type!("Expected type: array, but got: {}", s));
                        }
                    }
                    "prefixItems" => {
                        let prefix_items = load_array_of_schemas(value)?;
                        array_schema.prefix_items = Some(prefix_items);
                    }
                    _ => unimplemented!("Unsupported key: {}", key),
                }
            }
        }
        Ok(array_schema)
    }
}

impl Constructor<ConstSchema> for ConstSchema {
    fn construct(hash: &saphyr::Hash) -> Result<ConstSchema> {
        let const_key = sys("const");
        let value = hash.get(&const_key).unwrap();
        match value {
            saphyr::Yaml::String(s) => Ok(ConstSchema {
                r#const: ConstValue::string(s),
            }),
            saphyr::Yaml::Integer(i) => Ok(ConstSchema {
                r#const: ConstValue::integer(*i),
            }),
            saphyr::Yaml::Real(s) => {
                let f = s.parse::<f64>()?;
                Ok(ConstSchema {
                    r#const: ConstValue::float(f),
                })
            }
            _ => unimplemented!("Unsupported const value: {:#?}", value),
        }
    }
}

impl Constructor<IntegerSchema> for IntegerSchema {
    fn construct(hash: &saphyr::Hash) -> Result<IntegerSchema> {
        let mut integer_schema = IntegerSchema::default();
        for (key, value) in hash.iter() {
            if let saphyr::Yaml::String(key) = key {
                match key.as_str() {
                    "minimum" => {
                        integer_schema.minimum = Some(load_number(value)?);
                    }
                    "maximum" => {
                        integer_schema.maximum = Some(load_number(value)?);
                    }
                    "exclusiveMinimum" => {
                        integer_schema.exclusive_minimum = Some(load_number(value)?);
                    }
                    "exclusiveMaximum" => {
                        integer_schema.exclusive_maximum = Some(load_number(value)?);
                    }
                    "multipleOf" => {
                        integer_schema.multiple_of = Some(load_number(value)?);
                    }
                    "type" => {
                        let s = load_string_value(value)?;
                        if s != "integer" {
                            return Err(unsupported_type!(
                                "Expected type: integer, but got: {}",
                                s
                            ));
                        }
                    }
                    _ => unimplemented!("Unsupported key for type: integer: {}", key),
                }
            }
        }
        Ok(integer_schema)
    }
}

impl Constructor<EnumSchema> for EnumSchema {
    fn construct(hash: &saphyr::Hash) -> Result<EnumSchema> {
        let enum_key = sys("enum");
        let value = hash.get(&enum_key).unwrap();
        match value {
            saphyr::Yaml::Array(values) => {
                let enum_values = load_enum_values(values)?;
                Ok(EnumSchema {
                    r#enum: enum_values,
                })
            }
            v => Err(generic_error!("enum: Expected an array, but got: {:#?}", v)),
        }
    }
}

impl Constructor<ObjectSchema> for ObjectSchema {
    fn construct(hash: &saphyr::Hash) -> Result<ObjectSchema> {
        let mut object_schema = ObjectSchema::default();
        for (key, value) in hash.iter() {
            if let saphyr::Yaml::String(key) = key {
                match key.as_str() {
                    "properties" => {
                        let properties = load_properties(value.as_hash().unwrap())?;
                        object_schema.properties = Some(properties);
                    }
                    "additionalProperties" => {
                        let additional_properties = load_additional_properties(value)?;
                        object_schema.additional_properties = Some(additional_properties);
                    }
                    "minProperties" => {
                        object_schema.min_properties = Some(load_integer(value)? as usize);
                    }
                    "maxProperties" => {
                        object_schema.max_properties = Some(load_integer(value)? as usize);
                    }
                    "patternProperties" => {
                        let pattern_properties = load_properties(value.as_hash().unwrap())?;
                        object_schema.pattern_properties = Some(pattern_properties);
                    }
                    "propertyNames" => {
                        if !value.is_hash() {
                            return Err(unsupported_type!(
                                "propertyNames: Expected a hash, but got: {:?}",
                                value
                            ));
                        }
                        let hash = value.as_hash().unwrap();
                        if !hash.contains_key(&sys("pattern")) {
                            return Err(generic_error!(
                                "propertyNames: Missing required key: pattern"
                            ));
                        }
                        let pattern = load_string_value(hash.get(&sys("pattern")).unwrap())?;
                        object_schema.property_names = Some(pattern);
                    }
                    "anyOf" => {
                        if !value.is_array() {
                            return Err(unsupported_type!(
                                "anyOf: Expected an array, but got: {:?}",
                                value
                            ));
                        }

                        let mut any_of_schema = AnyOfSchema::default();
                        any_of_schema.any_of = load_array_of_schemas(value)?;
                        object_schema.any_of = Some(any_of_schema)
                    }
                    "required" => {
                        if !value.is_array() {
                            return Err(unsupported_type!(
                                "required: Expected an array, but got: {:?}",
                                value
                            ));
                        }
                        let array = value.as_vec().unwrap();
                        object_schema.required = Some(
                            array
                                .iter()
                                .map(|v| match v {
                                    saphyr::Yaml::String(s) => Ok(s.clone()),
                                    _ => Err(unsupported_type!(
                                        "required: Expected a string, but got: {:?}",
                                        v
                                    )),
                                })
                                .collect::<Result<Vec<String>>>()?,
                        );
                    }
                    "type" => {
                        let s = load_string_value(value)?;
                        if s != "object" {
                            return Err(unsupported_type!("Expected type: object, but got: {}", s));
                        }
                    }
                    _ => {
                        if key.starts_with("$") {
                            if object_schema.metadata.is_none() {
                                object_schema.metadata = Some(HashMap::new());
                            }
                            object_schema.metadata.as_mut().unwrap().insert(
                                key.clone(),
                                yaml_to_string(
                                    value,
                                    &format!("Value for {} must be a string", key),
                                )?,
                            );
                        } else {
                            unimplemented!("Unsupported key for type: object: {}", key);
                        }
                    }
                }
            }
        }
        Ok(object_schema)
    }
}

fn load_array_of_schemas(value: &saphyr::Yaml) -> Result<Vec<YamlSchema>> {
    match value {
        saphyr::Yaml::Array(values) => values
            .iter()
            .map(|v| match v {
                saphyr::Yaml::Hash(hash) => YamlSchema::construct(hash),
                _ => Err(unsupported_type!("Expected a hash, but got: {:?}", v)),
            })
            .collect::<Result<Vec<YamlSchema>>>(),
        _ => Err(unsupported_type!(
            "Expected an array of hashes, but got: {:?}",
            value
        )),
    }
}

impl Constructor<AnyOfSchema> for AnyOfSchema {
    fn construct(hash: &saphyr::Hash) -> Result<AnyOfSchema> {
        let mut any_of_schema = AnyOfSchema::default();
        for (key, value) in hash.iter() {
            if let saphyr::Yaml::String(key) = key {
                match key.as_str() {
                    "anyOf" => any_of_schema.any_of = load_array_of_schemas(value)?,
                    _ => return Err(unsupported_type!("Unsupported key: {}", key)),
                }
            }
        }
        Ok(any_of_schema)
    }
}

impl Constructor<OneOfSchema> for OneOfSchema {
    fn construct(hash: &saphyr::Hash) -> Result<OneOfSchema> {
        let mut one_of_schema = OneOfSchema::default();
        for (key, value) in hash.iter() {
            if let saphyr::Yaml::String(key) = key {
                match key.as_str() {
                    "oneOf" => {
                        one_of_schema.one_of = load_array_of_schemas(value)?;
                    }
                    _ => unimplemented!(),
                }
            }
        }
        Ok(one_of_schema)
    }
}

fn load_properties(hash: &saphyr::Hash) -> Result<HashMap<String, YamlSchema>> {
    let mut properties = HashMap::new();
    for (key, value) in hash.iter() {
        if let saphyr::Yaml::String(key) = key {
            if key.as_str() == "$ref" {
                let reference = Reference::construct(hash)?;
                properties.insert(key.clone(), YamlSchema::reference(reference));
            } else {
                let schema = YamlSchema::construct(value.as_hash().unwrap())?;
                properties.insert(key.clone(), schema);
            }
        } else {
            return Err(unsupported_type!(
                "Expected a string key, but got: {:?}",
                key
            ));
        }
    }
    Ok(properties)
}

fn load_additional_properties(value: &saphyr::Yaml) -> Result<BoolOrTypedSchema> {
    match value {
        saphyr::Yaml::Boolean(b) => Ok(BoolOrTypedSchema::Boolean(*b)),
        saphyr::Yaml::Hash(hash) => {
            let ref_key = sys("$ref");
            if hash.contains_key(&ref_key) {
                Ok(BoolOrTypedSchema::Reference(Reference::construct(hash)?))
            } else {
                let schema = TypedSchema::construct(hash)?;
                Ok(BoolOrTypedSchema::TypedSchema(Box::new(schema)))
            }
        }
        _ => Err(unsupported_type!(
            "Expected type: boolean or hash, but got: {:?}",
            value
        )),
    }
}

impl Constructor<NotSchema> for NotSchema {
    fn construct(hash: &saphyr::Hash) -> Result<NotSchema> {
        for (key, value) in hash.iter() {
            if let saphyr::Yaml::String(key) = key {
                match key.as_str() {
                    "not" => {
                        let schema = YamlSchema::construct(value.as_hash().unwrap())?;
                        return Ok(NotSchema {
                            not: Box::new(schema),
                        });
                    }
                    _ => unimplemented!(),
                }
            }
        }
        unimplemented!()
    }
}

fn load_integer(value: &saphyr::Yaml) -> Result<i64> {
    match value {
        saphyr::Yaml::Integer(i) => Ok(*i),
        _ => Err(unsupported_type!(
            "Expected type: integer, but got: {:?}",
            value
        )),
    }
}

fn load_number(value: &saphyr::Yaml) -> Result<Number> {
    match value {
        saphyr::Yaml::Integer(i) => Ok(Number::integer(*i)),
        saphyr::Yaml::Real(f) => Ok(Number::float(f.parse::<f64>()?)),
        _ => Err(unsupported_type!(
            "Expected type: integer or float, but got: {:?}",
            value
        )),
    }
}

impl Constructor<NumberSchema> for NumberSchema {
    fn construct(hash: &saphyr::Hash) -> Result<NumberSchema> {
        let mut number_schema = NumberSchema::default();
        for (key, value) in hash.iter() {
            if let saphyr::Yaml::String(key) = key {
                match key.as_str() {
                    "minimum" => {
                        let minimum = load_number(value).map_err(|_| {
                            crate::Error::UnsupportedType(format!(
                                "Expected type: integer or float, but got: {:?}",
                                &value
                            ))
                        })?;
                        number_schema.minimum = Some(minimum);
                    }
                    "maximum" => {
                        number_schema.maximum = Some(load_number(value)?);
                    }
                    "exclusiveMinimum" => {
                        number_schema.exclusive_minimum = Some(load_number(value)?);
                    }
                    "exclusiveMaximum" => {
                        number_schema.exclusive_maximum = Some(load_number(value)?);
                    }
                    "multipleOf" => {
                        number_schema.multiple_of = Some(load_number(value)?);
                    }
                    "type" => {
                        let s = load_string_value(value)?;
                        if s != "number" {
                            return Err(unsupported_type!("Expected type: number, but got: {}", s));
                        }
                    }
                    _ => unimplemented!(),
                }
            }
        }
        Ok(number_schema)
    }
}

impl Constructor<StringSchema> for StringSchema {
    fn construct(hash: &saphyr::Hash) -> Result<StringSchema> {
        let mut string_schema = StringSchema::default();
        for (key, value) in hash.iter() {
            if let saphyr::Yaml::String(key) = key {
                match key.as_str() {
                    "minLength" => {
                        if let saphyr::Yaml::Integer(i) = value {
                            string_schema.min_length = Some(*i as usize);
                        } else {
                            return Err(unsupported_type!(
                                "minLength expected integer, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "maxLength" => {
                        if let saphyr::Yaml::Integer(i) = value {
                            string_schema.max_length = Some(*i as usize);
                        } else {
                            return Err(unsupported_type!(
                                "maxLength expected integer, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "pattern" => {
                        if let saphyr::Yaml::String(s) = value {
                            let regex = regex::Regex::new(s.as_str())?;
                            string_schema.pattern = Some(regex);
                        } else {
                            return Err(unsupported_type!(
                                "pattern expected string, but got: {:?}",
                                value
                            ));
                        }
                    }
                    "type" => {
                        let s = load_string_value(value)?;
                        if s != "string" {
                            return Err(unsupported_type!("Expected type: string, but got: {}", s));
                        }
                    }
                    "enum" => {
                        if let saphyr::Yaml::Array(values) = value {
                            let enum_values = load_enum_values(values)?;
                            let string_enum_values = enum_values
                                .iter()
                                .map(|v| match v {
                                    ConstValue::String(s) => Ok(s.clone()),
                                    _ => Ok(format!("{}", v)),
                                })
                                .collect::<Result<Vec<String>>>()?;
                            string_schema.r#enum = Some(string_enum_values);
                        } else {
                            return Err(unsupported_type!(
                                "enum expected array, but got: {:?}",
                                value
                            ));
                        }
                    }
                    _ => unimplemented!("Unsupported key for type: string: {}", key),
                }
            }
        }
        Ok(string_schema)
    }
}

fn load_array_items(value: &saphyr::Yaml) -> Result<BoolOrTypedSchema> {
    match value {
        saphyr::Yaml::Boolean(b) => Ok(BoolOrTypedSchema::Boolean(*b)),
        saphyr::Yaml::Hash(hash) => {
            if hash.contains_key(&sys("$ref")) {
                let reference = Reference::construct(hash);
                Ok(BoolOrTypedSchema::Reference(reference?))
            } else if hash.contains_key(&sys("type")) {
                let typed_schema = TypedSchema::construct(hash)?;
                return Ok(BoolOrTypedSchema::TypedSchema(Box::new(typed_schema)));
            } else {
                return Err(generic_error!(
                    "Expected type: boolean or hash with type or $ref, but got: {:?}",
                    value
                ));
            }
        }
        _ => unimplemented!(),
    }
}

fn load_enum_values(values: &[saphyr::Yaml]) -> Result<Vec<ConstValue>> {
    Ok(values.iter().map(ConstValue::from_saphyr_yaml).collect())
}

fn yaml_to_string<S: Into<String> + Copy>(yaml: &saphyr::Yaml, msg: S) -> Result<String> {
    match yaml {
        saphyr::Yaml::String(s) => Ok(s.clone()),
        saphyr::Yaml::Integer(i) => Ok(i.to_string()),
        saphyr::Yaml::Real(f) => Ok(f.to_string()),
        saphyr::Yaml::Boolean(b) => Ok(b.to_string()),
        saphyr::Yaml::Null => Ok("null".to_string()),
        _ => Err(unsupported_type!(msg.into())),
    }
}

/// Convenience function to create saphyr::Yaml::String from a &str
fn sys(str: &str) -> saphyr::Yaml {
    saphyr::Yaml::String(String::from(str))
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    #[test]
    fn test_boolean_literal_true() {
        let root_schema = load_from_doc(&sys("true")).unwrap();
        assert_eq!(
            *root_schema.schema.as_ref(),
            YamlSchema::boolean_literal(true)
        );
    }

    #[test]
    fn test_boolean_literal_false() {
        let root_schema = load_from_doc(&sys("false")).unwrap();
        assert_eq!(
            *root_schema.schema.as_ref(),
            YamlSchema::boolean_literal(false)
        );
    }

    #[test]
    fn test_const_string() {
        let docs = saphyr::Yaml::load_from_str("const: string value").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let const_schema = ConstSchema {
            r#const: ConstValue::string("string value"),
        };
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Const(const_schema)
        );
    }

    #[test]
    fn test_const_integer() {
        let docs = saphyr::Yaml::load_from_str("const: 42").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let const_schema = ConstSchema {
            r#const: ConstValue::integer(42),
        };
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Const(const_schema)
        );
    }

    #[test]
    fn test_type_foo_should_error() {
        let docs = saphyr::Yaml::load_from_str("type: foo").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap());
        assert!(root_schema.is_err());
        assert_eq!(
            root_schema.unwrap_err().to_string(),
            "Unsupported type 'foo'!"
        );
    }

    #[test]
    fn test_type_string() {
        let docs = saphyr::Yaml::load_from_str("type: string").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let string_schema = StringSchema::default();
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::String(string_schema)
        );
    }

    #[test]
    fn test_type_object_with_string_with_description() {
        let docs = saphyr::Yaml::load_from_str(
            r#"
            type: object
            properties:
                name:
                    type: string
                    description: This is a description
        "#,
        )
        .unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let root_schema_schema = &root_schema.schema.as_ref().schema.as_ref().unwrap();
        let expected = StringSchema::default();
        if let Schema::Object(object_schema) = root_schema_schema {
            let name_property = object_schema
                .properties
                .as_ref()
                .expect("Expected properties")
                .get("name")
                .expect("Expected name property");
            let description = name_property
                .metadata
                .as_ref()
                .expect("Expected metadata")
                .get("description")
                .expect("Expected description");
            assert_eq!(description, "This is a description");
            if let Schema::String(actual) = &name_property.schema.as_ref().unwrap() {
                assert_eq!(&expected, actual);
            } else {
                panic!(
                    "Expected Schema::String, but got: {:?}",
                    name_property.schema
                );
            }
        } else {
            panic!("Expected Schema::Object, but got: {:?}", root_schema_schema);
        }
    }

    #[test]
    fn test_type_string_with_pattern() {
        let docs = saphyr::Yaml::load_from_str(
            r#"
        type: string
        pattern: "^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$"
        "#,
        )
        .unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let expected = StringSchema {
            pattern: Some(Regex::new("^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$").unwrap()),
            ..Default::default()
        };
        let root_schema_schema = root_schema.schema.as_ref().schema.as_ref().unwrap();
        assert_eq!(root_schema_schema, &Schema::String(expected));
    }

    #[test]
    fn test_array_constructor_items_true() {
        let mut hash = saphyr::Hash::new();
        hash.insert(sys("type"), sys("array"));
        hash.insert(sys("items"), saphyr::Yaml::Boolean(true));
        let array_schema = ArraySchema::construct(&hash).unwrap();
        assert_eq!(
            array_schema,
            ArraySchema {
                items: Some(BoolOrTypedSchema::Boolean(true)),
                prefix_items: None,
                contains: None
            }
        );
    }

    #[test]
    fn test_integer_schema() {
        let docs = saphyr::Yaml::load_from_str("type: integer").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let integer_schema = IntegerSchema::default();
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Integer(integer_schema)
        );
    }

    #[test]
    fn test_enum() {
        let docs = saphyr::Yaml::load_from_str(
            r#"
        enum:
          - foo
          - bar
          - baz
        "#,
        )
        .unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let enum_values = ["foo", "bar", "baz"]
            .iter()
            .map(|s| ConstValue::string(s.to_string()))
            .collect();
        let enum_schema = EnumSchema {
            r#enum: enum_values,
        };
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Enum(enum_schema)
        );
    }

    #[test]
    fn test_enum_without_type() {
        let docs = saphyr::Yaml::load_from_str(
            r#"
            enum:
              - red
              - amber
              - green
              - null
              - 42
            "#,
        )
        .unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let enum_values = vec![
            ConstValue::string("red".to_string()),
            ConstValue::string("amber".to_string()),
            ConstValue::string("green".to_string()),
            ConstValue::null(),
            ConstValue::integer(42),
        ];
        let enum_schema = EnumSchema {
            r#enum: enum_values,
        };
        assert_eq!(
            root_schema.schema.as_ref().schema.as_ref().unwrap(),
            &Schema::Enum(enum_schema)
        );
    }

    #[test]
    fn test_one_of_with_ref() {
        let docs = saphyr::Yaml::load_from_str(
            r##"
            $defs:
              foo:
                type: boolean
            oneOf:
              - type: string
              - $ref: "#/$defs/foo"
            "##,
        )
        .unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        println!("root_schema: {:#?}", root_schema);
        let root_schema_schema = root_schema.schema.as_ref().schema.as_ref().unwrap();
        if let Schema::OneOf(one_of_schema) = root_schema_schema {
            println!("one_of_schema: {:#?}", one_of_schema);
        } else {
            panic!("Expected Schema::OneOf, but got: {:?}", root_schema_schema);
        }

        let s = r#"
        false
        "#;
        let docs = saphyr::MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::with_root_schema(&root_schema, true);
        let result = root_schema.validate(&context, value);
        println!("result: {:#?}", result);
        assert!(result.is_ok());
        for error in context.errors.borrow().iter() {
            println!("error: {:#?}", error);
        }
        assert!(!context.has_errors());
    }
}
