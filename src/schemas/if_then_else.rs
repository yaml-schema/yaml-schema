//! JSON Schema-style `if` / `then` / `else` conditional validation.

use std::fmt::Display;

use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::YamlData;

use crate::Context;
use crate::Error;
use crate::Result;
use crate::Validator;
use crate::YamlSchema;

/// Conditional schema: `if` outcome selects `then` or `else`; `if` errors are not asserted on the parent.
#[derive(Debug, PartialEq)]
pub struct IfThenElseSchema {
    pub if_schema: Box<YamlSchema>,
    pub then_schema: Option<Box<YamlSchema>>,
    pub else_schema: Option<Box<YamlSchema>>,
}

impl Display for IfThenElseSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "if: {}", self.if_schema)?;
        if let Some(t) = &self.then_schema {
            write!(f, ", then: {t}")?;
        }
        if let Some(e) = &self.else_schema {
            write!(f, ", else: {e}")?;
        }
        Ok(())
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for IfThenElseSchema {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml<'r>) -> Result<Self> {
        if let YamlData::Mapping(mapping) = &value.data {
            IfThenElseSchema::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for IfThenElseSchema {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'r, MarkedYaml<'r>>) -> crate::Result<Self> {
        let if_key = MarkedYaml::value_from_str("if");
        let Some(if_value) = mapping.get(&if_key) else {
            return Err(generic_error!("No `if` key found for if/then/else"));
        };
        let if_schema: YamlSchema = if_value.try_into()?;

        let then_schema = mapping
            .get(&MarkedYaml::value_from_str("then"))
            .map(|v| v.try_into())
            .transpose()?
            .map(Box::new);

        let else_schema = mapping
            .get(&MarkedYaml::value_from_str("else"))
            .map(|v| v.try_into())
            .transpose()?
            .map(Box::new);

        Ok(IfThenElseSchema {
            if_schema: Box::new(if_schema),
            then_schema,
            else_schema,
        })
    }
}

impl Validator for IfThenElseSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> crate::Result<()> {
        debug!(
            "if/then/else: validating instance against `if` schema: {}",
            self.if_schema
        );
        let if_context = context.get_sub_context_fresh_eval();
        let if_result = self.if_schema.validate(&if_context, value);

        let if_passed = match if_result {
            Ok(()) | Err(Error::FailFast) => !if_context.has_errors(),
            Err(e) => return Err(e),
        };

        if if_passed {
            if let (Some(p), Some(f)) = (&context.object_evaluated, &if_context.object_evaluated) {
                p.extend(&f.snapshot());
            }
            if let (Some(pcell), Some(fcell)) =
                (&context.array_unevaluated, &if_context.array_unevaluated)
            {
                let snap = fcell.borrow().clone();
                pcell.borrow_mut().merge_from(&snap);
            }
            if let Some(then_s) = &self.then_schema {
                then_s.validate(context, value)?;
            }
        } else if let Some(else_s) = &self.else_schema {
            else_s.validate(context, value)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode;

    use crate::Context;
    use crate::Engine;
    use crate::Validator;
    use crate::loader;

    use super::*;

    #[test]
    fn if_passes_then_enforced() {
        let root = loader::load_from_str(
            r#"
            if:
              type: integer
            then:
              type: integer
              minimum: 5
            "#,
        )
        .unwrap();
        let ctx = Context::with_root_schema(&root, false);
        let v = MarkedYaml::load_from_str("7").unwrap();
        root.validate(&ctx, v.first().unwrap()).unwrap();
        assert!(!ctx.has_errors());

        let ctx = Context::with_root_schema(&root, false);
        let v = MarkedYaml::load_from_str("3").unwrap();
        root.validate(&ctx, v.first().unwrap()).unwrap();
        assert!(ctx.has_errors());
    }

    #[test]
    fn if_fails_else_enforced() {
        let root = loader::load_from_str(
            r#"
            if:
              type: integer
            else:
              type: string
              minLength: 2
            "#,
        )
        .unwrap();
        let ctx = Context::with_root_schema(&root, false);
        let v = MarkedYaml::load_from_str("\"ab\"").unwrap();
        root.validate(&ctx, v.first().unwrap()).unwrap();
        assert!(!ctx.has_errors());

        let ctx = Context::with_root_schema(&root, false);
        let v = MarkedYaml::load_from_str("\"x\"").unwrap();
        root.validate(&ctx, v.first().unwrap()).unwrap();
        assert!(ctx.has_errors());
    }

    #[test]
    fn if_fails_no_else_ok() {
        let root = loader::load_from_str(
            r#"
            if:
              type: string
            then:
              minLength: 10
            "#,
        )
        .unwrap();
        let ctx = Context::with_root_schema(&root, false);
        let v = MarkedYaml::load_from_str("42").unwrap();
        root.validate(&ctx, v.first().unwrap()).unwrap();
        assert!(
            !ctx.has_errors(),
            "if failed so then is skipped; instance should be valid"
        );
    }

    #[test]
    fn if_errors_not_reported_on_parent() {
        let root = loader::load_from_str(
            r#"
            if:
              type: string
            then:
              minLength: 100
            "#,
        )
        .unwrap();
        let ctx = Context::with_root_schema(&root, false);
        let v = MarkedYaml::load_from_str("42").unwrap();
        root.validate(&ctx, v.first().unwrap()).unwrap();
        assert!(
            !ctx.has_errors(),
            "failure of `if` must not surface as parent errors"
        );
    }

    #[test]
    fn type_and_conditional_both_apply() {
        let root = loader::load_from_str(
            r#"
            type: integer
            maximum: 10
            if:
              const: 5
            then:
              minimum: 0
            "#,
        )
        .unwrap();
        let ctx = Context::with_root_schema(&root, false);
        let v = MarkedYaml::load_from_str("20").unwrap();
        root.validate(&ctx, v.first().unwrap()).unwrap();
        assert!(
            ctx.has_errors(),
            "fails maximum even when if does not match and then is skipped"
        );
    }

    /// Examples from JSON Schema docs: if / then / else (postal codes).
    /// https://json-schema.org/understanding-json-schema/reference/conditionals#ifthenelse
    #[test]
    fn json_schema_doc_usa_canada_postal_if_then_else() {
        let schema = r#"
type: object
properties:
  street_address:
    type: string
  country:
    enum:
      - United States of America
      - Canada
if:
  type: object
  properties:
    country:
      const: "United States of America"
then:
  type: object
  properties:
    postal_code:
      type: string
      pattern: '^[0-9]{5}(-[0-9]{4})?$'
else:
  type: object
  properties:
    postal_code:
      type: string
      pattern: '^[A-Z][0-9][A-Z] [0-9][A-Z][0-9]$'
"#;
        let root = loader::load_from_str(schema).unwrap();

        let ok_usa = r#"
street_address: "1600 Pennsylvania Avenue NW"
country: "United States of America"
postal_code: "20500"
"#;
        assert!(!Engine::evaluate(&root, ok_usa, false).unwrap().has_errors());

        let ok_default_us = r#"
street_address: "1600 Pennsylvania Avenue NW"
postal_code: "20500"
"#;
        assert!(
            !Engine::evaluate(&root, ok_default_us, false)
                .unwrap()
                .has_errors()
        );

        let ok_ca = r#"
street_address: "24 Sussex Drive"
country: Canada
postal_code: "K1M 1M4"
"#;
        assert!(!Engine::evaluate(&root, ok_ca, false).unwrap().has_errors());

        let bad_ca = r#"
street_address: "24 Sussex Drive"
country: Canada
postal_code: "10000"
"#;
        assert!(Engine::evaluate(&root, bad_ca, false).unwrap().has_errors());

        let bad_wrong_zip = r#"
street_address: "1600 Pennsylvania Avenue NW"
postal_code: "K1M 1M4"
"#;
        assert!(
            Engine::evaluate(&root, bad_wrong_zip, false)
                .unwrap()
                .has_errors()
        );
    }

    #[test]
    fn json_schema_doc_allof_three_countries_postal() {
        let schema = r#"
type: object
properties:
  street_address:
    type: string
  country:
    enum:
      - United States of America
      - Canada
      - Netherlands
allOf:
  - if:
      type: object
      properties:
        country:
          const: "United States of America"
    then:
      type: object
      properties:
        postal_code:
          type: string
          pattern: '^[0-9]{5}(-[0-9]{4})?$'
  - if:
      type: object
      properties:
        country:
          const: Canada
      required:
        - country
    then:
      type: object
      properties:
        postal_code:
          type: string
          pattern: '^[A-Z][0-9][A-Z] [0-9][A-Z][0-9]$'
  - if:
      type: object
      properties:
        country:
          const: Netherlands
      required:
        - country
    then:
      type: object
      properties:
        postal_code:
          type: string
          pattern: '^[0-9]{4} [A-Z]{2}$'
"#;
        let root = loader::load_from_str(schema).unwrap();

        let ok_usa = r#"
street_address: "1600 Pennsylvania Avenue NW"
country: "United States of America"
postal_code: "20500"
"#;
        assert!(!Engine::evaluate(&root, ok_usa, false).unwrap().has_errors());

        let ok_default_us = r#"
street_address: "1600 Pennsylvania Avenue NW"
postal_code: "20500"
"#;
        assert!(
            !Engine::evaluate(&root, ok_default_us, false)
                .unwrap()
                .has_errors()
        );

        let ok_ca = r#"
street_address: "24 Sussex Drive"
country: Canada
postal_code: "K1M 1M4"
"#;
        assert!(!Engine::evaluate(&root, ok_ca, false).unwrap().has_errors());

        let nl = r#"
street_address: "Adriaan Goekooplaan"
country: Netherlands
postal_code: "2517 JX"
"#;
        assert!(!Engine::evaluate(&root, nl, false).unwrap().has_errors());

        let bad_ca = r#"
street_address: "24 Sussex Drive"
country: Canada
postal_code: "10000"
"#;
        assert!(Engine::evaluate(&root, bad_ca, false).unwrap().has_errors());

        let bad_wrong_zip = r#"
street_address: "1600 Pennsylvania Avenue NW"
postal_code: "K1M 1M4"
"#;
        assert!(
            Engine::evaluate(&root, bad_wrong_zip, false)
                .unwrap()
                .has_errors()
        );
    }
}
