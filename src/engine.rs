use saphyr::LoadableYamlNode;
use std::cell::RefCell;
use std::rc::Rc;

use crate::Error;
use crate::Result;
use crate::RootSchema;
use crate::Validator as _;
use crate::YamlSchema;
use crate::validation::Context;

#[derive(Debug)]
pub struct Engine<'a> {
    pub root_schema: &'a RootSchema<'a>,
    pub context: Rc<RefCell<Context<'a>>>,
}

impl<'a> Engine<'a> {
    pub fn new(root_schema: &'a RootSchema, context: Context<'a>) -> Self {
        Engine {
            root_schema,
            context: Rc::new(RefCell::new(context)),
        }
    }

    pub fn evaluate<'b: 'a>(
        root_schema: &'b RootSchema,
        value: &str,
        fail_fast: bool,
    ) -> Result<Context<'b>> {
        let context = Context::with_root_schema(root_schema, fail_fast);
        let engine = Engine::new(root_schema, context);
        let docs = saphyr::MarkedYaml::load_from_str(value).map_err(Error::YamlParsingError)?;
        match docs.first() {
            Some(yaml) => {
                engine
                    .root_schema
                    .validate(&engine.context.borrow(), yaml)?;
            }
            None => {
                // docs.is_empty()
                match &engine.root_schema.schema {
                    YamlSchema::Empty | YamlSchema::BooleanLiteral(true) => (),
                    // YamlSchema::Null or YamlSchema::Subschema(_)
                    _ => engine
                        .context
                        .borrow()
                        .add_doc_error("Empty YAML document is not allowed"),
                }
            }
        }
        Ok(engine.context.take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::YamlSchema;

    #[test]
    fn test_engine_empty_schema() {
        let root_schema = RootSchema::new(YamlSchema::Empty);
        let context = Engine::evaluate(&root_schema, "", false).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_engine_boolean_literal_true() {
        let root_schema = RootSchema::new(YamlSchema::BooleanLiteral(true));
        let context = Engine::evaluate(&root_schema, "", false).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_engine_boolean_literal_false() {
        let root_schema = RootSchema::new(YamlSchema::BooleanLiteral(false));
        let context = Engine::evaluate(&root_schema, "", false).unwrap();
        assert!(context.has_errors());
    }
}
