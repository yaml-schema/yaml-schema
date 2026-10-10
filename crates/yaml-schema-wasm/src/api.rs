use wasm_bindgen::JsError;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use yaml_schema::Engine;
use yaml_schema::loader;

use crate::result::FatalError;
use crate::result::Phase;
use crate::result::ValidationResult;

/// The YAML Schema meta-schema, used by [`validate_schema`].
const META_SCHEMA: &str = include_str!("../../../yaml-schema.yaml");

/// Options controlling validation. All fields are optional on the JS side; missing fields
/// fall back to the defaults.
#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ValidateOptions {
    /// Stop at the first validation error.
    pub fail_fast: bool,
}

fn parse_options(options: JsValue) -> Result<ValidateOptions, JsError> {
    if options.is_undefined() || options.is_null() {
        Ok(ValidateOptions::default())
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))
    }
}

fn to_js(result: &ValidationResult) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(result).map_err(|e| JsError::new(&e.to_string()))
}

fn validate_str(schema: &str, document: &str, fail_fast: bool) -> ValidationResult {
    let root_schema = match loader::load_from_str(schema) {
        Ok(root_schema) => root_schema,
        Err(e) => return ValidationResult::from_fatal(FatalError::new(Phase::Schema, &e)),
    };
    match Engine::evaluate(&root_schema, document, fail_fast) {
        Ok(context) => ValidationResult::from_errors(&context.errors.borrow()),
        Err(e @ yaml_schema::Error::YamlParsingError(_)) => {
            ValidationResult::from_fatal(FatalError::new(Phase::Document, &e))
        }
        Err(e) => ValidationResult::from_fatal(FatalError::new(Phase::Validation, &e)),
    }
}

fn validate_schema_str(schema: &str) -> ValidationResult {
    let mut result = validate_str(META_SCHEMA, schema, false);
    // The "document" here is the user's schema, so report parse errors against it as such.
    if let Some(fatal) = result.fatal.as_mut()
        && fatal.phase == Phase::Document
    {
        fatal.phase = Phase::Schema;
    }
    result
}

/// Validates a YAML `document` against a YAML `schema` (both as strings).
///
/// `options` is an optional `{ failFast?: boolean }` object.
///
/// Returns `{ valid, errors: [{ path, line, col, error }], fatal? }`, where `fatal` is
/// `{ phase: "schema" | "document" | "validation", message, line?, col? }` when validation
/// couldn't run to completion. Lines and columns are 1-based.
///
/// Remote (`http`/`https`) and `file://` `$ref`s aren't supported in the WebAssembly build;
/// they produce a `fatal` error with phase `"validation"`.
#[wasm_bindgen]
pub fn validate(schema: &str, document: &str, options: JsValue) -> Result<JsValue, JsError> {
    let options = parse_options(options)?;
    to_js(&validate_str(schema, document, options.fail_fast))
}

/// Validates a YAML `schema` against the YAML Schema meta-schema. Returns the same shape as
/// [`validate`]; YAML syntax errors in `schema` are reported with phase `"schema"`.
#[wasm_bindgen(js_name = validateSchema)]
pub fn validate_schema(schema: &str) -> Result<JsValue, JsError> {
    to_js(&validate_schema_str(schema))
}

/// Returns the version of the underlying `yaml-schema` library.
#[wasm_bindgen]
pub fn version() -> String {
    yaml_schema::version()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERSON_SCHEMA: &str = r#"
type: object
properties:
  name:
    type: string
  age:
    type: integer
    minimum: 0
required:
  - name
"#;

    #[test]
    fn test_valid_document() {
        let result = validate_str(PERSON_SCHEMA, "name: Alice\nage: 30\n", false);
        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(result.fatal.is_none());
    }

    #[test]
    fn test_invalid_document_reports_location() {
        let result = validate_str(PERSON_SCHEMA, "name: Alice\nage: -1\n", false);
        assert!(!result.valid);
        assert!(result.fatal.is_none());
        assert_eq!(result.errors.len(), 1);
        let issue = &result.errors[0];
        assert_eq!(issue.path, "age");
        assert_eq!(issue.line, Some(2));
        // `-1` starts at the 6th character of "age: -1"
        assert_eq!(issue.col, Some(6));
    }

    #[test]
    fn test_fail_fast_stops_at_first_error() {
        // Missing `name` and a negative `age`
        let all = validate_str(PERSON_SCHEMA, "age: -1\n", false);
        assert_eq!(all.errors.len(), 2);
        let fast = validate_str(PERSON_SCHEMA, "age: -1\n", true);
        assert_eq!(fast.errors.len(), 1);
        assert!(!fast.valid);
    }

    #[test]
    fn test_schema_yaml_error_is_schema_phase() {
        let result = validate_str("type: [object\n", "name: Alice\n", false);
        assert!(!result.valid);
        let fatal = result.fatal.expect("expected a fatal error");
        assert_eq!(fatal.phase, Phase::Schema);
        assert!(fatal.line.is_some());
    }

    #[test]
    fn test_unknown_schema_type_is_schema_phase() {
        let result = validate_str("type: frobnicate\n", "name: Alice\n", false);
        let fatal = result.fatal.expect("expected a fatal error");
        assert_eq!(fatal.phase, Phase::Schema);
    }

    #[test]
    fn test_document_yaml_error_is_document_phase() {
        let result = validate_str(PERSON_SCHEMA, "name: [Alice\n", false);
        assert!(!result.valid);
        let fatal = result.fatal.expect("expected a fatal error");
        assert_eq!(fatal.phase, Phase::Document);
        assert!(fatal.line.is_some());
        assert!(fatal.col.is_some());
    }

    #[test]
    fn test_internal_ref() {
        let schema = r##"
$defs:
  name:
    type: string
type: object
properties:
  name:
    $ref: "#/$defs/name"
"##;
        assert!(validate_str(schema, "name: Alice\n", false).valid);
        let result = validate_str(schema, "name: 42\n", false);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_relative_ref_without_base_uri_is_validation_phase() {
        let schema = "type: object\nproperties:\n  name:\n    $ref: other.yaml\n";
        let result = validate_str(schema, "name: Alice\n", false);
        let fatal = result.fatal.expect("expected a fatal error");
        assert_eq!(fatal.phase, Phase::Validation);
    }

    #[test]
    fn test_meta_schema_validates_itself() {
        let result = validate_schema_str(META_SCHEMA);
        assert!(result.valid, "{result:?}");
    }

    #[test]
    fn test_validate_schema() {
        assert!(validate_schema_str(PERSON_SCHEMA).valid);
        let result = validate_schema_str("type: object\nminProperties: lots\n");
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        let result = validate_schema_str("type: [object\n");
        assert_eq!(result.fatal.map(|f| f.phase), Some(Phase::Schema));
    }

    #[test]
    fn test_version() {
        assert_eq!(version(), yaml_schema::version());
    }
}
