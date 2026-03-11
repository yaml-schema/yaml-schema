use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::RootSchema;
use crate::YamlSchema;
use crate::validation::ValidationError;

/// The validation context
#[derive(Debug, Default)]
pub struct Context<'r> {
    /// We use an Option here so tests can be run without a root schema
    pub root_schema: Option<&'r RootSchema<'r>>,
    pub current_schema: Option<&'r YamlSchema<'r>>,
    pub current_path: Vec<String>,
    pub stream_started: bool,
    pub stream_ended: bool,
    pub errors: Rc<RefCell<Vec<ValidationError>>>,
    pub fail_fast: bool,
    /// Tracks `($ref, value_position)` pairs currently being resolved to detect circular references.
    /// The value position is the byte offset of the YAML value's span start, so the same ref
    /// applied to a nested value is allowed (legitimate recursion) while the same ref
    /// on the same value is detected as a cycle.
    pub resolving_refs: Rc<RefCell<HashSet<(String, usize)>>>,
}

impl<'r> Context<'r> {
    /// Returns true if there are any errors in the context
    pub fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// Returns the current path as a string separated by "."
    pub fn path(&self) -> String {
        self.current_path.join(".")
    }

    pub fn new(fail_fast: bool) -> Context<'r> {
        Context {
            fail_fast,
            ..Default::default()
        }
    }

    pub fn get_sub_context(&self) -> Context<'r> {
        Context {
            root_schema: self.root_schema,
            current_schema: self.current_schema,
            current_path: self.current_path.clone(),
            stream_started: self.stream_started,
            stream_ended: self.stream_ended,
            errors: Rc::new(RefCell::new(Vec::new())),
            fail_fast: self.fail_fast,
            resolving_refs: self.resolving_refs.clone(),
        }
    }

    pub fn with_root_schema(root_schema: &'r RootSchema, fail_fast: bool) -> Context<'r> {
        Context {
            root_schema: Some(root_schema),
            fail_fast,
            ..Default::default()
        }
    }

    fn push_error(&self, error: ValidationError) {
        self.errors.borrow_mut().push(error);
    }

    pub fn add_doc_error<V: Into<String>>(&self, error: V) {
        let path = self.path();
        self.push_error(ValidationError {
            path,
            marker: None,
            error: error.into(),
        });
    }

    /// Adds an error message to the current context, with the current path and with location marker
    pub fn add_error<V: Into<String>>(&self, marked_yaml: &saphyr::MarkedYaml, error: V) {
        let path = self.path();
        self.push_error(ValidationError {
            path,
            marker: Some(marked_yaml.span.start),
            error: error.into(),
        });
    }

    /// Appends all the errors to the current context
    pub fn extend_errors(&self, errors: Vec<ValidationError>) {
        self.errors.borrow_mut().extend(errors);
    }

    /// Append a path to the current path
    pub fn append_path<V: Into<String>>(&self, path: V) -> Context<'r> {
        let mut new_path = self.current_path.clone();
        new_path.push(path.into());
        Context {
            root_schema: self.root_schema,
            current_schema: self.current_schema,
            current_path: new_path,
            errors: self.errors.clone(),
            fail_fast: self.fail_fast,
            stream_ended: self.stream_ended,
            stream_started: self.stream_started,
            resolving_refs: self.resolving_refs.clone(),
        }
    }

    /// Returns `true` if the given ref is already being resolved for the given
    /// YAML value (identified by its span start index), indicating a cycle.
    pub fn is_resolving_ref(&self, ref_name: &str, value: &saphyr::MarkedYaml) -> bool {
        let key = (ref_name.to_string(), value.span.start.index());
        self.resolving_refs.borrow().contains(&key)
    }

    /// Mark a `(ref, value_position)` pair as currently being resolved.
    pub fn begin_resolving_ref(&self, ref_name: &str, value: &saphyr::MarkedYaml) {
        let key = (ref_name.to_string(), value.span.start.index());
        self.resolving_refs.borrow_mut().insert(key);
    }

    /// Remove a `(ref, value_position)` pair from the resolving set after resolution completes.
    pub fn end_resolving_ref(&self, ref_name: &str, value: &saphyr::MarkedYaml) {
        let key = (ref_name.to_string(), value.span.start.index());
        self.resolving_refs.borrow_mut().remove(&key);
    }
}
