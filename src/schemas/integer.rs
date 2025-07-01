use crate::validation::Context;
use crate::validation::Validator;
use crate::Number;
use crate::Result;

/// A number schema
#[derive(Debug, Default, PartialEq)]
pub struct IntegerSchema {
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

impl std::fmt::Display for IntegerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Integer {self:?}")
    }
}

impl Validator for IntegerSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        let data = &value.data;
        if let saphyr::YamlData::Value(scalar) = data {
            if let saphyr::Scalar::Integer(i) = scalar {
                crate::validation::validate_integer(
                    context,
                    &self.minimum,
                    &self.maximum,
                    &self.multiple_of,
                    &value,
                    *i,
                )
            } else if let saphyr::Scalar::FloatingPoint(o) = scalar {
                let f = o.into_inner();
                if f.fract() == 0.0 {
                    crate::validation::validate_integer(
                        context,
                        &self.minimum,
                        &self.maximum,
                        &self.multiple_of,
                        &value,
                        f as i64,
                    )
                } else {
                    context.add_error(value, format!("Expected an integer, but got: {data:?}"));
                }
            } else {
                context.add_error(value, format!("Expected a number, but got: {data:?}"));
            }
        } else {
            context.add_error(value, format!("Expected a scalar value, but got: {data:?}"));
        }
        if !context.errors.borrow().is_empty() {
            fail_fast!(context)
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saphyr::LoadableYamlNode;

    #[test]
    fn test_integer_schema_against_string() {
        let schema = IntegerSchema::default();
        let context = Context::new(true);
        let docs = saphyr::MarkedYaml::load_from_str("foo").unwrap();
        let result = schema.validate(&context, docs.first().unwrap());
        assert!(result.is_err());
        let errors = context.errors.borrow();
        assert!(!errors.is_empty());
        let first_error = errors.first().unwrap();
        assert_eq!(
            first_error.error,
            "Expected a number, but got: String(\"foo\")"
        );
    }
}
