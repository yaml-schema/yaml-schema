//! RootSchema represents the root document in a schema document.

use jsonptr::Pointer;
use log::debug;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;
use url::Url;

use crate::Error;
use crate::Result;
use crate::YamlSchema;
use crate::loader::marked_yaml_to_string;
use crate::validation::Context;
use crate::validation::Validator;

/// A RootSchema represents the root document in a schema document, and includes additional
/// fields such as `$schema` that are not allowed in subschemas. It also provides a way to
/// resolve references to other schemas.
#[derive(Debug, PartialEq)]
pub struct RootSchema {
    pub meta_schema: Option<String>,
    pub schema: YamlSchema,
    /// Base URI for resolving relative `$ref` values (from file path, URL, or `$id`).
    pub base_uri: Option<Url>,
}

impl RootSchema {
    /// Create an empty RootSchema
    pub fn empty() -> Self {
        Self {
            meta_schema: None,
            schema: YamlSchema::Empty,
            base_uri: None,
        }
    }

    /// Create a new RootSchema with a given schema
    pub fn new(schema: YamlSchema) -> Self {
        Self {
            meta_schema: None,
            schema,
            base_uri: None,
        }
    }

    /// Returns the `$id` of the schema's Subschema, if present.
    pub fn id(&self) -> Option<String> {
        match &self.schema {
            YamlSchema::Subschema(subschema) => subschema.metadata_and_annotations.id.clone(),
            _ => None,
        }
    }

    /// Returns the preferred key for caching this schema: `$id` if it is a valid URI,
    /// otherwise the given fallback (e.g. the file or fetch URI).
    pub fn cache_key(&self, fallback: &str) -> String {
        self.id()
            .filter(|s| Url::parse(s).is_ok())
            .unwrap_or_else(|| fallback.to_string())
    }

    /// Resolve a JSON Pointer to an element in the schema.
    pub fn resolve(&self, pointer: &Pointer) -> Option<&YamlSchema> {
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

impl<'r> TryFrom<&MarkedYaml<'r>> for RootSchema {
    type Error = Error;

    fn try_from(marked_yaml: &MarkedYaml<'r>) -> Result<Self> {
        match &marked_yaml.data {
            YamlData::Value(scalar) => match scalar {
                Scalar::Boolean(r#bool) => Ok(Self {
                    meta_schema: None,
                    schema: YamlSchema::BooleanLiteral(*r#bool),
                    base_uri: None,
                }),
                Scalar::Null => Ok(RootSchema {
                    meta_schema: None,
                    schema: YamlSchema::Null,
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

impl Validator for RootSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        self.schema.validate(context, value)
    }
}
