use std::cell::RefCell;
use std::rc::Rc;

use crate::validation::Context;
use crate::Error;
use crate::Result;
use crate::RootSchema;
use crate::Schema;

#[derive(Debug)]
pub struct Engine<'a> {
    pub root_schema: &'a RootSchema,
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
        if docs.is_empty() {
            if let Some(sub_schema) = &engine.root_schema.schema.as_ref().schema {
                match sub_schema {
                    Schema::Empty => (),
                    Schema::BooleanLiteral(false) => {
                        engine
                            .context
                            .borrow_mut()
                            .add_doc_error("Empty YAML document is not allowed");
                    }
                    Schema::BooleanLiteral(true) => (),
                    _ => engine
                        .context
                        .borrow_mut()
                        .add_doc_error("Empty YAML document is not allowed"),
                }
            }
        } else {
            let yaml = docs.first().unwrap();
            engine
                .root_schema
                .validate(&engine.context.borrow(), yaml)?;
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
        let root_schema = RootSchema::new(YamlSchema::empty());
        let context = Engine::evaluate(&root_schema, "", false).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_engine_boolean_literal_true() {
        let root_schema = RootSchema::new(YamlSchema::boolean_literal(true));
        let context = Engine::evaluate(&root_schema, "", false).unwrap();
        assert!(!context.has_errors());
    }

    #[test]
    fn test_engine_boolean_literal_false() {
        let root_schema = RootSchema::new(YamlSchema::boolean_literal(false));
        let context = Engine::evaluate(&root_schema, "", false).unwrap();
        assert!(context.has_errors());
    }
}
