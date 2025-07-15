use log::{debug, error};

use hashlink::LinkedHashMap;
use saphyr::{AnnotatedMapping, MarkedYaml, Scalar, YamlData};

use crate::loader;
use crate::loader::{marked_yaml_to_string, FromAnnotatedMapping, FromSaphyrMapping};
use crate::utils::{format_marker, format_yaml_data, linked_hash_map};
use crate::Context;
use crate::Error;
use crate::ObjectSchema;
use crate::Reference;
use crate::Schema;
use crate::StringSchema;
use crate::Validator;

/// YamlSchema is the core of the validation model
#[derive(Debug, Default, PartialEq)]
pub struct YamlSchema {
    pub metadata: Option<LinkedHashMap<String, String>>,
    pub r#ref: Option<Reference>,
    pub schema: Option<Schema>,
}

impl YamlSchema {
    pub fn empty() -> YamlSchema {
        YamlSchema {
            schema: Some(Schema::Empty),
            ..Default::default()
        }
    }

    pub fn null() -> YamlSchema {
        YamlSchema {
            schema: Some(Schema::TypeNull),
            ..Default::default()
        }
    }

    pub fn boolean_literal(value: bool) -> YamlSchema {
        YamlSchema {
            schema: Some(Schema::BooleanLiteral(value)),
            ..Default::default()
        }
    }

    pub fn object(object_schema: ObjectSchema) -> YamlSchema {
        YamlSchema {
            schema: Some(Schema::Object(Box::new(object_schema))),
            ..Default::default()
        }
    }

    pub fn reference(reference: Reference) -> YamlSchema {
        YamlSchema {
            r#ref: Some(reference),
            ..Default::default()
        }
    }

    pub fn ref_str<S>(ref_name: S) -> YamlSchema
    where
        S: Into<String>,
    {
        Self::reference(Reference::new(ref_name))
    }

    pub fn string() -> YamlSchema {
        YamlSchema {
            schema: Some(Schema::String(StringSchema::default())),
            ..Default::default()
        }
    }

    pub fn builder() -> YamlSchemaBuilder {
        YamlSchemaBuilder::new()
    }
}

impl From<Schema> for YamlSchema {
    fn from(schema: Schema) -> Self {
        YamlSchema {
            schema: Some(schema),
            ..Default::default()
        }
    }
}

impl TryFrom<&MarkedYaml<'_>> for YamlSchema {
    type Error = crate::Error;
    fn try_from(marked_yaml: &MarkedYaml<'_>) -> crate::Result<Self> {
        if let YamlData::Mapping(mapping) = &marked_yaml.data {
            let mut metadata: LinkedHashMap<String, String> = LinkedHashMap::new();
            let mut reference: Option<Reference> = None;
            let mut data = AnnotatedMapping::new();

            for (key, value) in mapping.iter() {
                match &key.data {
                    YamlData::Value(Scalar::String(s)) => match s.as_ref() {
                        "$id" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "$id must be a string")?,
                            );
                        }
                        "$schema" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "$schema must be a string")?,
                            );
                        }
                        "$ref" => match marked_yaml.try_into() {
                            Ok(r) => _ = reference.replace(r),
                            Err(_) => {
                                return Err(generic_error!(
                                    "[YamlSchema] Could not load as Reference: {:?}",
                                    marked_yaml
                                ))
                            }
                        },
                        "title" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "title must be a string")?,
                            );
                        }
                        "description" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "description must be a string")?,
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
            if let Some(reference) = reference {
                Ok(YamlSchema::reference(reference))
            } else {
                let my = MarkedYaml {
                    span: marked_yaml.span,
                    data: YamlData::Mapping(data),
                };
                let schema: Schema = (&my).try_into()?;
                Ok(YamlSchema {
                    metadata: if metadata.is_empty() {
                        None
                    } else {
                        Some(metadata)
                    },
                    schema: Some(schema),
                    r#ref: None,
                })
            }
        } else {
            Err(generic_error!(
                "{} Expected a mapping, but got: {:?}",
                format_marker(&marked_yaml.span.start),
                marked_yaml
            ))
        }
    }
}

impl FromSaphyrMapping<YamlSchema> for YamlSchema {
    fn from_mapping(mapping: &saphyr::Mapping) -> crate::Result<YamlSchema> {
        let mut metadata: LinkedHashMap<String, String> = LinkedHashMap::new();
        let mut r#ref: Option<Reference> = None;
        let mut data = saphyr::Mapping::new();

        for (key, value) in mapping.iter() {
            match key {
                saphyr::Yaml::Value(Scalar::String(s)) => {
                    match s.as_ref() {
                        "$id" => {
                            metadata.insert(
                                s.to_string(),
                                loader::yaml_to_string(value, "$id must be a string")?,
                            );
                        }
                        "$schema" => {
                            metadata.insert(
                                s.to_string(),
                                loader::yaml_to_string(value, "$schema must be a string")?,
                            );
                        }
                        "$ref" => {
                            r#ref = Some(Reference::from_mapping(mapping)?);
                            // TODO: What?
                        }
                        "title" => {
                            metadata.insert(
                                s.to_string(),
                                loader::yaml_to_string(value, "title must be a string")?,
                            );
                        }
                        "description" => {
                            metadata.insert(
                                s.to_string(),
                                loader::yaml_to_string(value, "description must be a string")?,
                            );
                        }
                        _ => {
                            data.insert(key.clone(), value.clone());
                        }
                    }
                }
                _ => {
                    data.insert(key.clone(), value.clone());
                }
            }
        }
        let schema = Schema::from_mapping(&data)?;
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

impl std::fmt::Display for YamlSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        if let Some(metadata) = &self.metadata {
            write!(f, "metadata: {metadata:?}, ")?;
        }
        if let Some(r#ref) = &self.r#ref {
            r#ref.fmt(f)?;
        }
        if let Some(schema) = &self.schema {
            write!(f, "schema: {schema}")?;
        }
        write!(f, "}}")
    }
}

pub struct YamlSchemaBuilder(YamlSchema);

impl Default for YamlSchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl YamlSchemaBuilder {
    pub fn new() -> Self {
        YamlSchemaBuilder(YamlSchema::default())
    }

    pub fn metadata<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        if let Some(metadata) = self.0.metadata.as_mut() {
            metadata.insert(key.into(), value.into());
        } else {
            self.0.metadata = Some(linked_hash_map(key.into(), value.into()));
        }
        self
    }

    pub fn description<S>(&mut self, description: S) -> &mut Self
    where
        S: Into<String>,
    {
        self.metadata("description", description)
    }

    pub fn r#ref(&mut self, r#ref: Reference) -> &mut Self {
        self.0.r#ref = Some(r#ref);
        self
    }

    pub fn schema(&mut self, schema: Schema) -> &mut Self {
        self.0.schema = Some(schema);
        self
    }

    pub fn string_schema(&mut self, string_schema: StringSchema) -> &mut Self {
        self.schema(Schema::String(string_schema))
    }

    pub fn object_schema(&mut self, object_schema: ObjectSchema) -> &mut Self {
        self.schema(Schema::Object(Box::new(object_schema)))
    }

    pub fn build(&mut self) -> YamlSchema {
        std::mem::take(&mut self.0)
    }
}

impl Validator for YamlSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> crate::Result<()> {
        debug!("[YamlSchema] self: {self}");
        debug!(
            "[YamlSchema] Validating value: {}",
            format_yaml_data(&value.data)
        );
        if let Some(reference) = &self.r#ref {
            debug!("[YamlSchema] Reference found: {reference}");
            let ref_name = &reference.ref_name;
            if let Some(root_schema) = context.root_schema {
                if let Some(schema) = root_schema.get_def(ref_name) {
                    debug!("[YamlSchema] Found {ref_name}: {schema}");
                    schema.validate(context, value)?;
                } else {
                    error!("[YamlSchema] Cannot find definition: {ref_name}");
                    context.add_error(value, format!("Schema {ref_name} not found"));
                }
            } else {
                return Err(generic_error!(
                    "YamlSchema has a reference, but no root schema was provided!"
                ));
            }
        } else if let Some(schema) = &self.schema {
            schema.validate(context, value)?;
        }
        Ok(())
    }
}

impl TryFrom<&AnnotatedMapping<'_, MarkedYaml<'_>>> for YamlSchema {
    type Error = Error;

    fn try_from(mapping: &AnnotatedMapping<'_, MarkedYaml<'_>>) -> crate::Result<Self> {
        let mut metadata: LinkedHashMap<String, String> = LinkedHashMap::new();
        let mut r#ref: Option<Reference> = None;
        let mut data = AnnotatedMapping::new();

        for (key, value) in mapping.iter() {
            match &key.data {
                YamlData::Value(Scalar::String(s)) => {
                    match s.as_ref() {
                        "$id" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "$id must be a string")?,
                            );
                        }
                        "$schema" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "$schema must be a string")?,
                            );
                        }
                        "$ref" => {
                            r#ref = Some(value.try_into()?);
                            // TODO: What?
                        }
                        "title" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "title must be a string")?,
                            );
                        }
                        "description" => {
                            metadata.insert(
                                s.to_string(),
                                marked_yaml_to_string(value, "description must be a string")?,
                            );
                        }
                        _ => {
                            data.insert(key.clone(), value.clone());
                        }
                    }
                }
                _ => {
                    data.insert(key.clone(), value.clone());
                }
            }
        }
        let schema = Some(Schema::from_annotated_mapping(&data)?);
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
