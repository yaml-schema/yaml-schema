use crate::reference::Reference;
use crate::TypedSchema;

#[derive(Debug, PartialEq)]
pub enum BoolOrTypedSchema {
    Boolean(bool),
    TypedSchema(Box<TypedSchema>),
    Reference(Reference),
}

impl std::fmt::Display for BoolOrTypedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoolOrTypedSchema::Boolean(b) => write!(f, "{}", b),
            BoolOrTypedSchema::TypedSchema(s) => s.fmt(f),
            BoolOrTypedSchema::Reference(r) => r.fmt(f),
        }
    }
}
