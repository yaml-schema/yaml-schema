use thiserror::Error;

use crate::loader::UrlLoadError;

/// Unexpected errors that can occur during the validation of a YAML schema
#[derive(Debug, Error)]
pub enum Error {
    #[error("Generic YAML schema error: {0}")]
    GenericError(String),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error(transparent)]
    YamlParsingError(#[from] saphyr::ScanError),
    #[error(transparent)]
    FloatParsingError(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    RegexParsingError(#[from] regex::Error),
    #[error("Error loading schema: {0}")]
    SchemaLoadingError(String),
    #[error("Unsupported type: {0}")]
    UnsupportedType(String),
    #[error("{0} Expected mapping, but got: {1}")]
    ExpectedMapping(String, String),
    #[error("Expected YAML scalar: {0}")]
    ExpectedScalar(String),
    #[error("{0} Expected a string value for `type:`, but got: {1}")]
    ExpectedTypeIsString(String, String),
    #[error("Fail fast signal")]
    FailFast,
    #[error("Invalid regular expression: {0}")]
    InvalidRegularExpression(String),
    #[error(transparent)]
    UrlLoadError(#[from] UrlLoadError),
    #[error(transparent)]
    JsonPtrError(#[from] jsonptr::ParseError),
    #[error("Not yet implemented!")]
    NotYetImplemented,
}

#[macro_export]
macro_rules! fail_fast {
    ($context:expr) => {
        if $context.fail_fast {
            return Err($crate::Error::FailFast);
        }
    };
}

#[macro_export]
macro_rules! schema_loading_error {
    ($s:literal, $($e:expr),+) => {
        $crate::Error::SchemaLoadingError(format!($s, $($e),+))
    };
    ($s:literal) => {
        $crate::Error::SchemaLoadingError($s.to_string())
    };
}

#[macro_export]
macro_rules! unsupported_type {
    ($s:literal, $($e:expr),+) => {
        $crate::Error::UnsupportedType(format!($s, $($e),+))
    };
    ($e:expr) => {
        $crate::Error::UnsupportedType()
    };
}

#[macro_export]
macro_rules! generic_error {
    ($s:literal, $($e:expr),+) => {
        $crate::Error::GenericError(format!($s, $($e),+))
    };
    ($s:literal) => {
        $crate::Error::GenericError($s.to_string())
    };
}

#[macro_export]
macro_rules! expected_mapping {
    ($marked_yaml:expr) => {
        $crate::Error::ExpectedMapping(
            $crate::utils::format_marker(&$marked_yaml.span.start),
            format!("{:?}", $marked_yaml.data),
        )
    };
}

#[macro_export]
macro_rules! expected_scalar {
    ($s:literal, $($e:expr),+) => {
        $crate::Error::ExpectedScalar(format!($s, $($e),+))
    };
    ($s:literal) => {
        $crate::Error::ExpectedScalar($s.to_string())
    };
}

#[macro_export]
macro_rules! expected_type_is_string {
    ($marked_yaml:expr) => {
        $crate::Error::ExpectedTypeIsString(
            $crate::utils::format_marker(&$marked_yaml.span.start),
            format!("{:?}", $marked_yaml.data),
        )
    };
}
