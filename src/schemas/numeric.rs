use std::cmp::Ordering;

use saphyr::MarkedYaml;

use crate::Number;
use crate::validation::Context;

/// Shared numeric bound constraints used by both `IntegerSchema` and `NumberSchema`.
#[derive(Debug, Default, PartialEq)]
pub struct NumericBounds {
    pub minimum: Option<Number>,
    pub maximum: Option<Number>,
    pub exclusive_minimum: Option<Number>,
    pub exclusive_maximum: Option<Number>,
    pub multiple_of: Option<Number>,
}

impl NumericBounds {
    /// Try to parse `key`/`value` as one of the numeric bound keys
    /// (`minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`,
    /// `multipleOf`), assigning into `self` if recognized.
    /// Returns `Ok(true)` if the key was consumed, `Ok(false)` otherwise.
    pub fn try_apply_key(&mut self, key: &str, value: &MarkedYaml) -> crate::Result<bool> {
        match key {
            "minimum" => self.minimum = Some(value.try_into()?),
            "maximum" => self.maximum = Some(value.try_into()?),
            "exclusiveMinimum" => self.exclusive_minimum = Some(value.try_into()?),
            "exclusiveMaximum" => self.exclusive_maximum = Some(value.try_into()?),
            "multipleOf" => self.multiple_of = Some(value.try_into()?),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Validate `actual` against all configured bounds, reporting errors to `context`.
    pub fn validate(&self, context: &Context, value: &MarkedYaml, actual: Number) {
        if let Some(exclusive_min) = self.exclusive_minimum
            && actual.partial_cmp(&exclusive_min) != Some(Ordering::Greater)
        {
            context.add_error(
                value,
                format!("Number must be greater than {exclusive_min}"),
            );
        }
        if let Some(minimum) = self.minimum
            && actual < minimum
        {
            context.add_error(
                value,
                format!("Number must be greater than or equal to {minimum}"),
            );
        }

        if let Some(exclusive_max) = self.exclusive_maximum
            && actual.partial_cmp(&exclusive_max) != Some(Ordering::Less)
        {
            context.add_error(value, format!("Number must be less than {exclusive_max}"));
        }
        if let Some(maximum) = self.maximum
            && actual > maximum
        {
            context.add_error(
                value,
                format!("Number must be less than or equal to {maximum}"),
            );
        }

        if let Some(multiple) = self.multiple_of
            && !actual.is_multiple_of(multiple)
        {
            context.add_error(value, format!("Number is not a multiple of {multiple}!"));
        }
    }
}
