//! The serializable result types returned to JS.
//!
//! All `line`/`col` values are **1-based**, ready for use by editors. (saphyr's markers have
//! 1-based lines but 0-based columns; see `yaml_schema::validation::ValidationError`.)

use serde::Serialize;
use yaml_schema::validation::ValidationError;

/// The outcome of a validation run.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    /// `true` when there were no validation errors and no fatal error.
    pub valid: bool,
    /// Validation errors found in the document.
    pub errors: Vec<ValidationIssue>,
    /// Set when validation could not run to completion (e.g. the schema or document isn't
    /// valid YAML, or the schema couldn't be loaded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fatal: Option<FatalError>,
}

impl ValidationResult {
    pub fn from_errors(errors: &[ValidationError]) -> Self {
        let errors: Vec<ValidationIssue> = errors.iter().map(ValidationIssue::from).collect();
        Self {
            valid: errors.is_empty(),
            errors,
            fatal: None,
        }
    }

    pub fn from_fatal(fatal: FatalError) -> Self {
        Self {
            valid: false,
            errors: Vec::new(),
            fatal: Some(fatal),
        }
    }
}

/// A single validation error, located in the validated document.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    /// The path to the offending value, e.g. `foo.bar[0]`.
    pub path: String,
    /// 1-based line number.
    pub line: Option<usize>,
    /// 1-based column number.
    pub col: Option<usize>,
    /// The error message.
    pub error: String,
}

impl From<&ValidationError> for ValidationIssue {
    fn from(e: &ValidationError) -> Self {
        Self {
            path: e.path.clone(),
            line: e.marker.map(|m| m.line()),
            col: e.marker.map(|m| m.col() + 1),
            error: e.error.clone(),
        }
    }
}

/// Which input a fatal error relates to, so a UI can attach it to the right editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// The schema isn't valid YAML, or couldn't be loaded as a schema.
    Schema,
    /// The document isn't valid YAML.
    Document,
    /// Validation aborted (e.g. an unresolvable `$ref`).
    Validation,
}

/// An error that prevented validation from completing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FatalError {
    pub phase: Phase,
    pub message: String,
    /// 1-based line number, when known.
    pub line: Option<usize>,
    /// 1-based column number, when known.
    pub col: Option<usize>,
}

impl FatalError {
    pub fn new(phase: Phase, error: &yaml_schema::Error) -> Self {
        match error {
            yaml_schema::Error::YamlParsingError(scan_error) => {
                let marker = scan_error.marker();
                Self {
                    phase,
                    message: scan_error.info().to_string(),
                    line: Some(marker.line()),
                    col: Some(marker.col() + 1),
                }
            }
            other => Self {
                phase,
                message: other.to_string(),
                line: None,
                col: None,
            },
        }
    }
}
