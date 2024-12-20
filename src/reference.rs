/// A RefSchema is a reference to another schema, usually one that is
/// declared in the `$defs` section of the root schema.
use log::debug;
use std::rc::Rc;

use crate::loader::Constructor;
use crate::Result;
use crate::YamlSchema;

#[derive(Debug, Default, PartialEq)]
pub struct Reference {
    pub ref_name: String,
    pub referenced_schema: Option<Rc<YamlSchema>>,
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
            referenced_schema: None,
        }
    }
}

impl Constructor<Reference> for Reference {
    fn construct(hash: &saphyr::Hash) -> Result<Reference> {
        debug!("[Reference] hash: {:#?}", hash);
        let ref_key = saphyr::Yaml::String(String::from("$ref"));
        if !hash.contains_key(&ref_key) {
            return Err(generic_error!("Expected a $ref key, but got: {:#?}", hash));
        }

        let ref_value = hash.get(&ref_key).unwrap();
        match ref_value {
            saphyr::Yaml::String(s) => Ok(Reference::new(s)),
            _ => Err(generic_error!(
                "Expected a string value for $ref, but got: {:#?}",
                ref_value
            )),
        }
    }
}
