use crate::Result;
/// A RefSchema is a reference to another schema, usually one that is
/// declared in the `$defs` section of the root schema.
use crate::loader::FromSaphyrMapping;
use crate::utils::{format_marker, saphyr_yaml_string};
use saphyr::{MarkedYaml, YamlData};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reference {
    pub ref_name: String,
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$ref: {}", self.ref_name)
    }
}

impl Reference {
    pub fn new<S>(ref_name: S) -> Reference
    where
        S: Into<String>,
    {
        Reference {
            ref_name: ref_name.into(),
        }
    }
}

impl TryFrom<&MarkedYaml<'_>> for Reference {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml<'_>) -> std::result::Result<Self, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
            let ref_key = MarkedYaml::value_from_str("$ref");
            if !mapping.contains_key(&ref_key) {
                return Err(generic_error!(
                    "{} Expected a $ref key, but got: {:#?}",
                    format_marker(&value.span.start),
                    mapping
                ));
            }

            let ref_value = mapping.get(&ref_key).unwrap();
            match &ref_value.data {
                YamlData::Value(saphyr::Scalar::String(s)) => {
                    if !s.starts_with("#/$defs/") && !s.starts_with("#/definitions/") {
                        return Err(generic_error!(
                            "Only local references, starting with #/$defs/ or #/definitions/ are supported for now. Found: {}",
                            s
                        ));
                    }
                    let ref_name = match s.strip_prefix("#/$defs/") {
                        Some(ref_name) => ref_name,
                        _ => s.strip_prefix("#/definitions/").unwrap(),
                    };

                    Ok(Reference::new(ref_name))
                }
                _ => Err(generic_error!(
                    "Expected a string value for $ref, but got: {:#?}",
                    ref_value
                )),
            }
        } else {
            Err(generic_error!(
                "{} value is not a mapping: {:?}",
                format_marker(&value.span.start),
                value
            ))
        }
    }
}

impl FromSaphyrMapping<Reference> for Reference {
    fn from_mapping(hash: &saphyr::Mapping) -> Result<Reference> {
        let ref_key = saphyr_yaml_string("$ref");
        if !hash.contains_key(&ref_key) {
            return Err(generic_error!("Expected a $ref key, but got: {:#?}", hash));
        }

        let ref_value = hash.get(&ref_key).unwrap();
        match ref_value {
            saphyr::Yaml::Value(saphyr::Scalar::String(s)) => {
                if !s.starts_with("#/$defs/") && !s.starts_with("#/definitions/") {
                    return Err(generic_error!(
                        "Only local references, starting with #/$defs/ or #/definitions/ are supported for now. Found: {}",
                        s
                    ));
                }
                let ref_name = match s.strip_prefix("#/$defs/") {
                    Some(ref_name) => ref_name,
                    _ => s.strip_prefix("#/definitions/").unwrap(),
                };

                Ok(Reference::new(ref_name))
            }
            _ => Err(generic_error!(
                "Expected a string value for $ref, but got: {:#?}",
                ref_value
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RootSchema;
    use saphyr::LoadableYamlNode;

    #[test]
    fn test_reference_constructor() {
        let mut hash = saphyr::Mapping::new();
        hash.insert(
            saphyr_yaml_string("$ref"),
            saphyr_yaml_string("#/$defs/name"),
        );
        let reference = Reference::from_mapping(&hash).unwrap();
        println!("reference: {reference:#?}");
        assert_eq!("name", reference.ref_name);
    }

    #[test]
    fn test_reference() {
        let schema = r##"
            $defs:
                name:
                    type: string
            type: object
            properties:
                name:
                    $ref: "#/$defs/name"
        "##;
        let root_schema = RootSchema::load_from_str(schema).unwrap();
        let yaml_schema = root_schema.schema.as_ref();
        println!("yaml_schema: {yaml_schema:#?}");
        let schema = yaml_schema.schema.as_ref().unwrap();
        println!("schema: {schema:#?}");
        if let crate::Schema::Object(object_schema) = schema {
            if let Some(properties) = &object_schema.properties {
                if let Some(name_property) = properties.get("name") {
                    let name_ref = name_property.r#ref.as_ref().unwrap();
                    assert_eq!(name_ref.ref_name, "name");
                }
            }
        }
        let context = crate::Context::with_root_schema(&root_schema, true);
        let value = r##"
            name: "John Doe"
        "##;
        let docs = saphyr::MarkedYaml::load_from_str(value).unwrap();
        let value = docs.first().unwrap();
        let result = root_schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(!context.has_errors());
    }
}
