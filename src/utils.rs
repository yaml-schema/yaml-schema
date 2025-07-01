// Various utility functions
use std::borrow::Cow;

use crate::Result;

/// Construct a saphyr::Yaml scalar value from a &str
pub const fn saphyr_yaml_string(s: &str) -> saphyr::Yaml<'_> {
    saphyr::Yaml::Value(saphyr::Scalar::String(Cow::Borrowed(s)))
}

/// Try to unwrap a saphyr::Scalar from a saphyr::Yaml
pub fn try_unwrap_saphyr_scalar<'a>(yaml: &'a saphyr::Yaml) -> Result<&'a saphyr::Scalar<'a>> {
    if let saphyr::Yaml::Value(scalar) = yaml {
        Ok(scalar)
    } else {
        Err(expected_scalar!("Expected a scalar, got: {:?}", yaml))
    }
}
