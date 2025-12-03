use std::borrow::Cow;
use std::fmt::Display;

use hashlink::LinkedHashMap;
use jsonptr::Token;
use log::debug;
use log::error;
use saphyr::{AnnotatedMapping, MarkedYaml, Scalar, YamlData};

use crate::ConstValue;
use crate::Context;
use crate::Error;
use crate::Reference;
use crate::Result;
use crate::Validator;
use crate::loader::marked_yaml_to_string;
use crate::schemas::AllOfSchema;
use crate::schemas::AnyOfSchema;
use crate::schemas::ArraySchema;
use crate::schemas::IntegerSchema;
use crate::schemas::NotSchema;
use crate::schemas::NumberSchema;
use crate::schemas::ObjectSchema;
use crate::schemas::OneOfSchema;
use crate::schemas::StringSchema;
use crate::utils::format_annotated_mapping;
use crate::utils::format_linked_hash_map;
use crate::utils::format_marked_yaml;
use crate::utils::format_marker;
use crate::utils::format_scalar;
use crate::utils::format_vec;
use crate::utils::format_yaml_data;

/// YamlSchema is the base of the validation model
#[derive(Debug, PartialEq)]
pub enum YamlSchema<'r> {
    Empty,                // no value
    Null,                 // `null`
    BooleanLiteral(bool), // `true` or `false`
    Subschema(Box<Subschema<'r>>),
}

impl<'r> YamlSchema<'r> {
    pub fn subschema(subschema: Subschema<'r>) -> Self {
        Self::Subschema(Box::new(subschema))
    }

    pub fn ref_str(ref_name: impl Into<Cow<'r, str>>) -> Self {
        Self::subschema(Subschema {
            r#ref: Some(Reference::new(ref_name.into())),
            ..Default::default()
        })
    }

    /// Create a YamlSchema with a single type: `boolean`
    pub fn typed_boolean() -> Self {
        Self::subschema(Subschema {
            r#type: Some(SchemaType::Single("boolean".to_string())),
            ..Default::default()
        })
    }

    /// Create a YamlSchema with a single type: `number`
    pub fn typed_number(number_schema: NumberSchema) -> Self {
        number_schema.into()
    }

    /// Create a YamlSchema with a single type: `string`
    pub fn typed_string(string_schema: StringSchema) -> Self {
        Self::subschema(Subschema {
            r#type: Some(SchemaType::Single("string".to_string())),
            string_schema: Some(string_schema),
            ..Default::default()
        })
    }

    /// Create a YamlSchema with a single type: `object`
    pub fn typed_object(object_schema: ObjectSchema<'r>) -> Self {
        Self::subschema(Subschema {
            r#type: Some(SchemaType::Single("object".to_string())),
            object_schema: Some(object_schema),
            ..Default::default()
        })
    }

    /// Resolve a portion of a JSON Pointer to an element in the schema.
    pub fn resolve(
        &self,
        key: Option<&Token>,
        components: &[jsonptr::Component],
    ) -> Option<&YamlSchema<'_>> {
        debug!("[YamlSchema#resolve] self: {self}, key: {key:?}, components: {components:?}");
        if components.is_empty() {
            return Some(self);
        }
        match self {
            YamlSchema::Subschema(subschema) => subschema.resolve(key, components),
            _ => None,
        }
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for YamlSchema<'r> {
    type Error = crate::Error;
    fn try_from(marked_yaml: &MarkedYaml<'r>) -> crate::Result<Self> {
        match &marked_yaml.data {
            YamlData::Value(scalar) => match scalar {
                Scalar::Boolean(value) => Ok(YamlSchema::BooleanLiteral(*value)),
                Scalar::Null => Ok(YamlSchema::Null),
                _ => Err(generic_error!(
                    "[YamlSchema#try_from] Expected a boolean or null, but got: {}",
                    format_scalar(scalar)
                )),
            },
            YamlData::Mapping(_) => Subschema::try_from(marked_yaml).map(YamlSchema::subschema),
            _ => Err(generic_error!(
                "[YamlSchema#try_from] Expected a boolean, null, or a mapping, but got: {}",
                format_marked_yaml(marked_yaml)
            )),
        }
    }
}

impl<'r> From<NumberSchema> for YamlSchema<'r> {
    fn from(number_schema: NumberSchema) -> Self {
        YamlSchema::subschema(Subschema {
            r#type: Some(SchemaType::Single("number".to_string())),
            number_schema: Some(number_schema),
            ..Default::default()
        })
    }
}

impl<'r> From<IntegerSchema> for YamlSchema<'r> {
    fn from(integer_schema: IntegerSchema) -> Self {
        YamlSchema::subschema(Subschema {
            r#type: Some(SchemaType::Single("integer".to_string())),
            integer_schema: Some(integer_schema),
            ..Default::default()
        })
    }
}

impl<'r> From<StringSchema> for YamlSchema<'r> {
    fn from(string_schema: StringSchema) -> Self {
        YamlSchema::subschema(Subschema {
            r#type: Some(SchemaType::Single("string".to_string())),
            string_schema: Some(string_schema),
            ..Default::default()
        })
    }
}

impl Validator for YamlSchema<'_> {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[YamlSchema] self: {self}");
        debug!(
            "[YamlSchema] Validating value: {}",
            format_yaml_data(&value.data)
        );
        match self {
            YamlSchema::Empty => Ok(()),
            YamlSchema::Null => {
                if !matches!(&value.data, YamlData::Value(Scalar::Null)) {
                    context.add_error(
                        value,
                        format!("Expected null, but got: {}", format_yaml_data(&value.data)),
                    );
                }
                Ok(())
            }
            YamlSchema::BooleanLiteral(boolean) => {
                if !*boolean {
                    context.add_error(value, "YamlSchema is `false`!");
                }
                Ok(())
            }
            YamlSchema::Subschema(subschema) => {
                debug!("[YamlSchema#validate] Validating subschema: {subschema:?}");
                subschema.validate(context, value)?;
                Ok(())
            }
        }
    }
}

impl<'r> From<Subschema<'r>> for YamlSchema<'r> {
    fn from(subschema: Subschema<'r>) -> Self {
        YamlSchema::subschema(subschema)
    }
}

impl Display for YamlSchema<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlSchema::Empty => write!(f, "<empty>"),
            YamlSchema::Null => write!(f, "null"),
            YamlSchema::BooleanLiteral(value) => write!(f, "{value}"),
            YamlSchema::Subschema(subschema) => subschema.fmt(f),
        }
    }
}

/// Represents either a literal boolean value or a YamlSchema
#[derive(Debug, PartialEq)]
pub enum BooleanOrSchema<'r> {
    Boolean(bool),
    Schema(YamlSchema<'r>),
}

impl BooleanOrSchema<'_> {
    pub fn schema<'r>(schema: YamlSchema<'r>) -> BooleanOrSchema<'r> {
        BooleanOrSchema::Schema(schema)
    }
}

impl Display for BooleanOrSchema<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BooleanOrSchema::Boolean(value) => write!(f, "{value}"),
            BooleanOrSchema::Schema(schema) => schema.fmt(f),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SchemaType {
    Single(String),
    Multiple(Vec<String>),
}

impl SchemaType {
    pub fn single<S: Into<String>>(value: S) -> Self {
        SchemaType::Single(value.into())
    }
    pub fn is_single(&self) -> bool {
        matches!(self, SchemaType::Single(_))
    }
    pub fn is_multiple(&self) -> bool {
        matches!(self, SchemaType::Multiple(_))
    }
}

impl Display for SchemaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaType::Single(value) => write!(f, "{value}"),
            SchemaType::Multiple(values) => write!(f, "{}", format_vec(values)),
        }
    }
}

/// A Subschema contains the core schema elements and validation
#[derive(Debug, Default, PartialEq)]
pub struct Subschema<'r> {
    /// `$id` and `$schema` metadata and `title` and `description` annotations
    pub metadata_and_annotations: MetadataAndAnnotations,
    /// `$anchor` metadata
    pub anchor: Option<String>,
    /// `$ref`
    pub r#ref: Option<Reference<'r>>,
    /// `$defs`
    pub defs: Option<LinkedHashMap<String, YamlSchema<'r>>>,
    /// `anyOf`
    pub any_of: Option<AnyOfSchema<'r>>,
    /// `allOf`
    pub all_of: Option<AllOfSchema<'r>>,
    /// `oneOf`
    pub one_of: Option<OneOfSchema<'r>>,
    /// `not`
    pub not: Option<NotSchema<'r>>,
    /// `type`
    pub r#type: Option<SchemaType>,
    /// `const`
    pub r#const: Option<ConstValue>,
    /// `enum`
    pub r#enum: Option<Vec<ConstValue>>,

    pub array_schema: Option<ArraySchema<'r>>,
    pub integer_schema: Option<IntegerSchema>,
    pub number_schema: Option<NumberSchema>,
    pub object_schema: Option<ObjectSchema<'r>>,
    pub string_schema: Option<StringSchema>,
}

impl<'r> Subschema<'r> {
    /// Resolve a portion of a JSON Pointer to an element in the schema.
    pub fn resolve(
        &self,
        token: Option<&Token>,
        components: &[jsonptr::Component],
    ) -> Option<&YamlSchema<'_>> {
        debug!("[Subschema#resolve] self: {self}, token: {token:?}, components: {components:?}");
        if let Some(token) = token {
            let s = token.decoded();
            debug!("[Subschema#resolve] key: {s}");
            match s.as_ref() {
                "$defs" => {
                    debug!("[Subschema#resolve] Resolving $defs");
                    if let Some(defs) = self.defs.as_ref() {
                        debug!("[Subschema#resolve] defs: {:?}", defs);
                        if let Some(component) = components.first() {
                            debug!("[Subschema#resolve] component: {component:?}");
                            if let jsonptr::Component::Token(next_token) = component {
                                let decoded = next_token.decoded();
                                debug!("[Subschema#resolve] decoded: {decoded}");
                                debug!("[Subschema#resolve] defs: {defs:?}");
                                if let Some(schema) = defs.get(decoded.as_ref()) {
                                    debug!("[Subschema#resolve] schema: {schema:?}");
                                    return schema.resolve(Some(next_token), &components[1..]);
                                }
                            }
                        }
                    }
                }
                "anyOf" => {}
                _ => (),
            }
        }
        None
    }
}

// Try to load a Subschema from a MarkedYaml. Delegate to the TryFrom<&AnnotatedMapping<'_>> for mappings.
// If the MarkedYaml is not a mapping, returns an error.
impl<'r> TryFrom<&MarkedYaml<'r>> for Subschema<'r> {
    type Error = crate::Error;
    fn try_from(marked_yaml: &MarkedYaml<'r>) -> crate::Result<Self> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            Self::try_from(mapping)
        } else {
            Err(generic_error!(
                "{} Expected a mapping, but got: {:?}",
                format_marker(&marked_yaml.span.start),
                marked_yaml
            ))
        }
    }
}

fn try_load_defs<'r>(
    marked_yaml: &MarkedYaml<'r>,
) -> Result<LinkedHashMap<String, YamlSchema<'r>>> {
    debug!(
        "[try_load_defs] marked_yaml: {}",
        format_yaml_data(&marked_yaml.data)
    );
    if let YamlData::Mapping(mapping) = &marked_yaml.data {
        debug!(
            "[try_load_defs] mapping: {}",
            format_annotated_mapping(mapping)
        );
        mapping
            .iter()
            .try_fold(LinkedHashMap::new(), |mut acc, (key, value)| {
                let key = marked_yaml_to_string(key, "key must be a string")?;
                acc.insert(key, value.try_into()?);
                Ok(acc)
            })
    } else {
        Err(expected_mapping!(marked_yaml))
    }
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for Subschema<'r> {
    type Error = Error;

    fn try_from(mapping: &AnnotatedMapping<'r, MarkedYaml<'r>>) -> crate::Result<Self> {
        debug!(
            "[Subschema#try_from] mapping has {} keys",
            mapping.keys().len()
        );
        for key in mapping.keys() {
            debug!("[Subschema#try_from] key: {:?}", key.data);
        }

        let metadata_and_annotations = MetadataAndAnnotations::try_from(mapping)?;
        debug!("[Subschema#try_from] metadata_and_annotations: {metadata_and_annotations}");

        // $defs
        let defs: Option<LinkedHashMap<String, YamlSchema<'r>>> = mapping
            .get(&MarkedYaml::value_from_str("$defs"))
            .map(|x| {
                debug!("[Subschema#try_from] x: {}", format_yaml_data(&x.data));
                debug!("[Subschema#try_from] Trying to load `$defs` as LinkedHashMap<String, YamlSchema<'r>>");
                try_load_defs(x)
            })
            .transpose()?;

        // $ref
        let reference: Option<Reference> = mapping
            .get(&MarkedYaml::value_from_str("$ref"))
            .map(|_| {
                debug!("[Subschema#try_from] Trying to load `$ref` as Reference");
                mapping.try_into()
            })
            .transpose()?;

        // anyOf
        let any_of: Option<AnyOfSchema> = mapping
            .get(&MarkedYaml::value_from_str("anyOf"))
            .map(|_| {
                debug!("[Subschema#try_from] Trying to load `anyOf` as AnyOfSchema");
                mapping.try_into()
            })
            .transpose()?;

        // allOf
        let all_of: Option<AllOfSchema> = mapping
            .get(&MarkedYaml::value_from_str("allOf"))
            .map(|_| {
                debug!("[Subschema#try_from] Trying to load `allOf` as AllOfSchema");
                mapping.try_into()
            })
            .transpose()?;

        // oneOf
        let one_of: Option<OneOfSchema> = mapping
            .get(&MarkedYaml::value_from_str("oneOf"))
            .map(|_| {
                debug!("[Subschema#try_from] Trying to load `oneOf` as OneOfSchema");
                mapping.try_into()
            })
            .transpose()?;

        // not
        let not: Option<NotSchema> = mapping
            .get(&MarkedYaml::value_from_str("not"))
            .map(|_| {
                debug!("[Subschema#try_from] Trying to load `not` as NotSchema");
                mapping.try_into()
            })
            .transpose()?;

        // const
        let mut r#const: Option<ConstValue> = None;
        if let Some(value) = mapping.get(&MarkedYaml::value_from_str("const")) {
            r#const = Some(ConstValue::try_from(value)?);
        }

        // enum
        let mut r#enum: Option<Vec<ConstValue>> = None;
        if let Some(value) = mapping.get(&MarkedYaml::value_from_str("enum"))
            && let saphyr::YamlData::Sequence(values) = &value.data
        {
            let enum_values = values
                .iter()
                .map(|marked_yaml| marked_yaml.try_into())
                .collect::<Result<Vec<ConstValue>>>()?;
            r#enum = Some(enum_values);
        }

        // type
        let mut r#type: Option<SchemaType> = None;
        if let Some(type_value) = mapping.get(&MarkedYaml::value_from_str("type")) {
            match &type_value.data {
                YamlData::Value(Scalar::String(s)) => {
                    r#type = Some(SchemaType::Single(s.to_string()))
                }
                YamlData::Sequence(values) => {
                    r#type = Some(SchemaType::Multiple(
                        values
                            .iter()
                            .map(|marked_yaml| {
                                marked_yaml_to_string(marked_yaml, "type must be a string")
                            })
                            .collect::<Result<Vec<String>>>()?,
                    ))
                }
                _ => {
                    return Err(schema_loading_error!(
                        "[Subschema#try_from] Expected a string or sequence for `type`, but got: {:?}",
                        type_value.data
                    ));
                }
            }
        }

        // Instantiate the appropriate schema based on the type
        let mut array_schema = None;
        let mut integer_schema = None;
        let mut number_schema = None;
        let mut object_schema = None;
        let mut string_schema = None;
        if let Some(type_value) = &r#type
            && let SchemaType::Single(s) = type_value
        {
            match s.as_ref() {
                "array" => {
                    array_schema = ArraySchema::try_from(mapping).ok();
                }
                "boolean" => {}
                "integer" => {
                    integer_schema = IntegerSchema::try_from(mapping).ok();
                }
                "number" => {
                    number_schema = NumberSchema::try_from(mapping).ok();
                }
                "object" => {
                    object_schema = ObjectSchema::try_from(mapping).ok();
                }
                "string" => {
                    string_schema = StringSchema::try_from(mapping).ok();
                }
                _ => {
                    return Err(unsupported_type!(
                        "Expected type: string, number, integer, object, or array, but got: {}",
                        s
                    ));
                }
            }
        }

        Ok(Self {
            metadata_and_annotations,
            defs,
            r#ref: reference,
            any_of,
            all_of,
            one_of,
            not,
            r#type,
            r#const,
            r#enum,
            array_schema,
            integer_schema,
            number_schema,
            object_schema,
            string_schema,
            anchor: None,
        })
    }
}

impl Display for Subschema<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        if !self.metadata_and_annotations.is_empty() {
            write!(f, " ")?;
            self.metadata_and_annotations.fmt(f)?;
            write!(f, " ")?;
        }
        if let Some(r#type) = &self.r#type {
            write!(f, "type: ")?;
            r#type.fmt(f)?;
        }
        if let Some(r#ref) = &self.r#ref {
            write!(f, "$ref: ")?;
            r#ref.fmt(f)?;
        }
        if let Some(defs) = &self.defs {
            write!(f, "$defs: {}", format_linked_hash_map(defs))?;
        }
        if let Some(any_of) = &self.any_of {
            write!(f, "anyOf: ")?;
            any_of.fmt(f)?;
        }
        if let Some(all_of) = &self.all_of {
            write!(f, "allOf: ")?;
            all_of.fmt(f)?;
        }
        if let Some(one_of) = &self.one_of {
            write!(f, "oneOf: ")?;
            one_of.fmt(f)?;
        }
        if let Some(not) = &self.not {
            write!(f, "not: ")?;
            not.fmt(f)?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl Validator for Subschema<'_> {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> crate::Result<()> {
        debug!("[Subschema] self: {self}");
        debug!(
            "[Subschema] Validating value: {}",
            format_yaml_data(&value.data)
        );
        if let Some(reference) = &self.r#ref {
            debug!("[Subschema] Reference found: {reference}");
            let ref_name = &reference.ref_name;
            if let Some(root_schema) = context.root_schema {
                if let Some(ref_name) = ref_name.strip_prefix("#") {
                    let pointer =
                        jsonptr::Pointer::parse(ref_name).expect("Failed to parse reference name");
                    debug!("[Subschema] Pointer: {pointer}");
                    let schema = root_schema.resolve(pointer);
                    if let Some(schema) = schema {
                        debug!("[Subschema] Found {ref_name}: {schema}");
                        schema.validate(context, value)?;
                    } else {
                        error!("[Subschema] Cannot find definition: {ref_name}");
                        context.add_error(value, format!("Schema {ref_name} not found"));
                    }
                } else {
                    error!("[Subschema] Cannot find definition: {ref_name}");
                    context.add_error(value, format!("Schema {ref_name} not found"));
                }
            } else {
                return Err(generic_error!(
                    "Subschema has a reference, but no root schema was provided!"
                ));
            }
        }

        if let Some(string_schema) = &self.string_schema {
            debug!("[Subschema] Validating string schema: {string_schema:?}");
            string_schema.validate(context, value)?;
        }

        if let Some(number_schema) = &self.number_schema {
            debug!("[Subschema] Validating number schema: {number_schema:?}");
            number_schema.validate(context, value)?;
        }

        if let Some(integer_schema) = &self.integer_schema {
            debug!("[Subschema] Validating integer schema: {integer_schema:?}");
            integer_schema.validate(context, value)?;
        }

        if let Some(object_schema) = &self.object_schema {
            debug!("[Subschema] Validating object schema: {object_schema:?}");
            object_schema.validate(context, value)?;
        }

        if let Some(any_of) = &self.any_of {
            debug!("[Subschema] Validating anyOf schema: {any_of:?}");
            any_of.validate(context, value)?;
        }

        if let Some(all_of) = &self.all_of {
            debug!("[Subschema] Validating allOf schema: {all_of:?}");
            all_of.validate(context, value)?;
        }

        if let Some(one_of) = &self.one_of {
            debug!("[Subschema] Validating oneOf schema: {one_of:?}");
            one_of.validate(context, value)?;
        }

        if let Some(not) = &self.not {
            debug!("[Subschema] Validating not schema: {not:?}");
            not.validate(context, value)?;
        }

        Ok(())
    }
}

/// The `$id` and `$schema` metadata
#[derive(Debug, Default, PartialEq)]
pub struct MetadataAndAnnotations {
    /// `$id` metadata
    pub id: Option<String>,
    /// `$schema` metadata
    pub schema: Option<String>,
    /// `title` annotation
    pub title: Option<String>,
    /// `description` annotation
    pub description: Option<String>,
}

impl MetadataAndAnnotations {
    pub fn is_empty(&self) -> bool {
        self.id.is_none()
            && self.schema.is_none()
            && self.title.is_none()
            && self.description.is_none()
    }
}

impl std::fmt::Display for MetadataAndAnnotations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        if !self.is_empty() {
            write!(f, " ")?;
            if let Some(id) = &self.id {
                write!(f, "id: {id}, ")?;
            }
            if let Some(schema) = &self.schema {
                write!(f, "schema: {schema}, ")?;
            }
            if let Some(title) = &self.title {
                write!(f, "title: {title}, ")?;
            }
            if let Some(description) = &self.description {
                write!(f, "description: {description}, ")?;
            }
            write!(f, " ")?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for MetadataAndAnnotations {
    type Error = Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let mut metadata_and_annotations = MetadataAndAnnotations::default();
        for (key, value) in mapping.iter() {
            match &key.data {
                YamlData::Value(Scalar::String(s)) => match s.as_ref() {
                    "$id" => {
                        metadata_and_annotations.id =
                            Some(marked_yaml_to_string(value, "$id must be a string")?);
                    }
                    "$schema" => {
                        metadata_and_annotations.schema =
                            Some(marked_yaml_to_string(value, "$schema must be a string")?);
                    }
                    "title" => {
                        metadata_and_annotations.title =
                            Some(marked_yaml_to_string(value, "title must be a string")?);
                    }
                    "description" => {
                        metadata_and_annotations.description = Some(marked_yaml_to_string(
                            value,
                            "description must be a string",
                        )?);
                    }
                    _ => {
                        debug!("[MetadataAndAnnotations#try_from] Unknown key: {s}");
                    }
                },
                _ => {
                    debug!("[MetadataAndAnnotations#try_from] Unsupported key data: {key:?}");
                }
            }
        }
        Ok(metadata_and_annotations)
    }
}

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode;

    use super::*;

    #[test]
    fn test_type_boolean() {
        let yaml = r#"
        type: boolean
        "#;
        let doc = MarkedYaml::load_from_str(yaml).expect("Failed to load YAML");
        let marked_yaml = doc.first().unwrap();
        let yaml_schema = YamlSchema::try_from(marked_yaml).unwrap();
        let YamlSchema::Subschema(subschema) = yaml_schema else {
            panic!("Expected a subschema");
        };
        assert!(subschema.r#type.is_some());
        let types = subschema.r#type.expect("Expected a type");
        assert!(types.is_single());
        let SchemaType::Single(type_value) = types else {
            panic!("Expected a single type");
        };
        assert_eq!(type_value, "boolean");
    }

    #[test]
    fn test_metadata_and_annotations_try_from() {
        let yaml = r#"
        $id: http://example.com/schema
        $schema: http://example.com/schema
        title: Example Schema
        description: This is an example schema
        "#;
        let doc = MarkedYaml::load_from_str(yaml).expect("Failed to load YAML");
        let marked_yaml = doc.first().unwrap();
        assert!(marked_yaml.data.is_mapping());
        let YamlData::Mapping(mapping) = &marked_yaml.data else {
            panic!("Expected a mapping");
        };
        let metadata_and_annotations = MetadataAndAnnotations::try_from(mapping).unwrap();
        assert_eq!(
            metadata_and_annotations.id,
            Some("http://example.com/schema".to_string())
        );
        assert_eq!(
            metadata_and_annotations.schema,
            Some("http://example.com/schema".to_string())
        );
        assert_eq!(
            metadata_and_annotations.title,
            Some("Example Schema".to_string())
        );
        assert_eq!(
            metadata_and_annotations.description,
            Some("This is an example schema".to_string())
        );
    }

    #[test]
    fn test_yaml_schema_with_multiple_types() {
        let yaml = r#"
        type:
          - boolean
          - number
          - integer
          - string
        "#;
        let doc = MarkedYaml::load_from_str(yaml).expect("Failed to load YAML");
        let marked_yaml = doc.first().unwrap();
        let yaml_schema = YamlSchema::try_from(marked_yaml).unwrap();
        let YamlSchema::Subschema(subschema) = yaml_schema else {
            panic!("Expected a subschema");
        };
        assert!(subschema.r#type.is_some());
        let types = subschema.r#type.expect("Expected a type");
        assert!(types.is_multiple());
        let SchemaType::Multiple(type_values) = types else {
            panic!("Expected a multiple type");
        };
        assert_eq!(type_values, vec!["boolean", "number", "integer", "string"]);
    }
}
