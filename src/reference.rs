use std::borrow::Cow;

use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::YamlData;

use crate::utils::format_annotated_mapping;

/// A Reference is a reference to another schema, usually one that is
/// declared in the `$defs` section of the root schema.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reference<'r> {
    pub ref_name: Cow<'r, str>,
}

impl std::fmt::Display for Reference<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$ref: {}", self.ref_name)
    }
}

impl<'r> Reference<'r> {
    pub fn new(ref_name: Cow<'r, str>) -> Reference<'r> {
        Reference { ref_name }
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for Reference<'r> {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml<'r>) -> std::result::Result<Self, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
            Self::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for Reference<'r> {
    type Error = crate::Error;

    fn try_from<'a>(
        mapping: &AnnotatedMapping<'a, MarkedYaml<'a>>,
    ) -> crate::Result<Reference<'a>> {
        debug!("[Reference#try_from] {}", format_annotated_mapping(mapping));
        let ref_key = MarkedYaml::value_from_str("$ref");
        if let Some(ref_value) = mapping.get(&ref_key) {
            match &ref_value.data {
                YamlData::Value(saphyr::Scalar::String(s)) => {
                    if !s.starts_with("#/$defs/") && !s.starts_with("#/definitions/") {
                        return Err(generic_error!(
                            "Only local references, starting with #/$defs/ or #/definitions/ are supported for now. Found: {}",
                            s
                        ));
                    }
                    Ok(Reference::new(s.clone()))
                }
                _ => Err(generic_error!(
                    "Expected a string value for $ref, but got: {:?}",
                    ref_value
                )),
            }
        } else {
            Err(generic_error!(
                "No $ref key found in mapping: {}",
                format_annotated_mapping(mapping)
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Validator as _;
    use crate::YamlSchema;
    use crate::loader;
    use saphyr::LoadableYamlNode;

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
        let root_schema = loader::load_from_str(schema).expect("Failed to load schema");
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected a subschema");
        };
        println!("subschema: {subschema:?}");
        // TODO: assert that the subschema has the expected structure

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

    #[test]
    fn test_json_ptr() {
        let ptr = jsonptr::Pointer::parse("/$defs/schema").expect("Failed to parse JSON pointer");
        println!("ptr: {ptr:?}");
        for component in ptr.components() {
            println!("component: {component:?}");
        }
    }
}
