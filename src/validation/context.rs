use std::cell::RefCell;
use std::rc::Rc;

use crate::validation::ValidationError;

/// The validation context
pub struct Context {
    pub current_path: Vec<String>,
    pub errors: Rc<RefCell<Vec<ValidationError>>>,
    pub fail_fast: bool,
}

impl Context {
    /// Returns true if there are any errors in the context
    pub fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// Returns the current path as a string separated by "."
    pub fn path(&self) -> String {
        self.current_path.join(".")
    }

    pub fn new(fail_fast: bool) -> Context {
        Context {
            current_path: vec![],
            errors: Rc::new(RefCell::new(Vec::new())),
            fail_fast,
        }
    }

    pub fn push_error(&self, error: ValidationError) {
        self.errors.borrow_mut().push(error);
    }

    pub fn add_error<V: Into<String>>(&self, error: V) {
        let path = self.path();
        self.push_error(ValidationError {
            path,
            error: error.into(),
        });
    }

    /// Append a path to the current path
    pub fn append_path<V: Into<String>>(&self, path: V) -> Context {
        let mut new_path = self.current_path.clone();
        new_path.push(path.into());
        Context {
            current_path: new_path,
            errors: self.errors.clone(),
            fail_fast: self.fail_fast,
        }
    }
}
