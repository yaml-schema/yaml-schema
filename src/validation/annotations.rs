//! Annotation state for JSON Schema 2020-12 `unevaluatedProperties` / `unevaluatedItems`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Successfully evaluated object property names at one instance (for `unevaluatedProperties`).
#[derive(Debug, Clone, Default)]
pub struct ObjectEvaluatedNames {
    pub names: Rc<RefCell<HashSet<String>>>,
}

impl ObjectEvaluatedNames {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, name: impl Into<String>) {
        self.names.borrow_mut().insert(name.into());
    }

    pub fn extend(&self, other: &HashSet<String>) {
        self.names.borrow_mut().extend(other.iter().cloned());
    }

    pub fn snapshot(&self) -> HashSet<String> {
        self.names.borrow().clone()
    }
}

/// Tracks keywords that feed `unevaluatedItems` at one array instance (JSON Schema 2020-12 §11.2).
#[derive(Debug, Clone, Default)]
pub struct ArrayUnevaluatedAnnotations {
    /// Any annotation from prefixItems, items, contains, unevaluatedItems, or merged in-place applicators.
    pub saw_relevant: bool,
    /// Boolean true from `items` or nested `unevaluatedItems` → ignore unevaluatedItems.
    pub full_coverage: bool,
    /// Largest index prefixItems applied a subschema to (`None` if prefixItems absent or empty).
    pub prefix_largest: Option<usize>,
    /// Indices where contains matched successfully.
    pub contains_indices: HashSet<usize>,
    /// Contains produced boolean true (every index).
    pub contains_all: bool,
}

impl ArrayUnevaluatedAnnotations {
    pub fn new_shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::default()))
    }

    /// Indices that must still be validated by `unevaluatedItems` (JSON Schema 2020-12 §11.2).
    pub fn indices_requiring_unevaluated(&self, len: usize) -> Vec<usize> {
        if self.full_coverage {
            return Vec::new();
        }
        if !self.saw_relevant {
            return (0..len).collect();
        }
        let start = self
            .prefix_largest
            .map(|p| p.saturating_add(1))
            .unwrap_or(0);
        let mut out = Vec::new();
        for i in start..len {
            if self.contains_all || self.contains_indices.contains(&i) {
                continue;
            }
            out.push(i);
        }
        out
    }

    /// Merge another branch's annotations (e.g. successful `anyOf` sibling). Union semantics.
    pub fn merge_from(&mut self, other: &ArrayUnevaluatedAnnotations) {
        if other.saw_relevant {
            self.saw_relevant = true;
        }
        if other.full_coverage {
            self.full_coverage = true;
        }
        self.prefix_largest = match (self.prefix_largest, other.prefix_largest) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        if other.contains_all {
            self.contains_all = true;
        }
        self.contains_indices
            .extend(other.contains_indices.iter().copied());
    }
}
