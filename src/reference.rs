/// A RefSchema is a reference to another schema, usually one that is
/// declared in the `$defs` section of the root schema.
use crate::loader::Constructor;
use crate::Result;

#[derive(Debug, Default, PartialEq)]
pub struct Reference {
    pub ref_name: String,
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$ref: {}", self.ref_name)
    }
}

impl Reference {
    pub fn new<S: Into<String>>(ref_name: S) -> Reference {
        Reference {
            ref_name: ref_name.into(),
        }
    }
}

impl Constructor<Reference> for Reference {
    fn construct(hash: &saphyr::Hash) -> Result<Reference> {
        let ref_key = saphyr::Yaml::String(String::from("$ref"));
        if !hash.contains_key(&ref_key) {
            return Err(generic_error!("Expected a $ref key, but got: {:#?}", hash));
        }

        let ref_value = hash.get(&ref_key).unwrap();
        match ref_value {
            saphyr::Yaml::String(s) => {
                if !s.starts_with("#/$defs/") {
                    return Err(generic_error!("Only local references, starting with #/$defs/ are supported for now. Found: {}", s));
                }
                let ref_name = s.strip_prefix("#/$defs/").unwrap();
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

    #[test]
    fn test_reference_constructor() {
        let mut hash = saphyr::Hash::new();
        hash.insert(
            saphyr::Yaml::String(String::from("$ref")),
            saphyr::Yaml::String(String::from("#/$defs/name")),
        );
        let reference = Reference::construct(&hash).unwrap();
        println!("reference: {:#?}", reference);
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
        println!("yaml_schema: {:#?}", yaml_schema);
        let schema = yaml_schema.schema.as_ref().unwrap();
        println!("schema: {:#?}", schema);
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
