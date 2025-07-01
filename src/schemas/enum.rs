use log::debug;

use crate::ConstValue;
use crate::Context;
use crate::Error;
use crate::Result;
use crate::Validator;
use crate::{format_vec, format_yaml_data};

/// An enum schema represents a set of constant values
#[derive(Debug, Default, PartialEq)]
pub struct EnumSchema {
    pub r#enum: Vec<ConstValue>,
}

impl std::fmt::Display for EnumSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Enum {{ enum: {} }}", format_vec(&self.r#enum))
    }
}

impl Validator for EnumSchema {
    fn validate(&self, context: &Context, value: &saphyr::MarkedYaml) -> Result<()> {
        debug!("[EnumSchema] self: {}", self);
        let data = &value.data;
        debug!("[EnumSchema] Validating value: {:?}", data);
        let const_value: ConstValue = data.try_into().map_err(|_| {
            Error::GenericError(format!("Unable to convert value: {data:?} to ConstValue"))
        })?;
        debug!("[EnumSchema] const_value: {}", const_value);
        for value in &self.r#enum {
            debug!("[EnumSchema] value: {}", value);
            if value.eq(&const_value) {
                return Ok(());
            }
        }
        if !self.r#enum.contains(&const_value) {
            let value_str = format_yaml_data(data);
            let enum_values = self
                .r#enum
                .iter()
                .map(|v| format!("{v}"))
                .collect::<Vec<String>>()
                .join(", ");
            let error = format!("Value {value_str} is not in the enum: [{enum_values}]");
            debug!("[EnumSchema] error: {}", error);
            context.add_error(value, error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saphyr::LoadableYamlNode;

    #[test]
    fn test_enum_schema() {
        let schema = EnumSchema {
            r#enum: vec![ConstValue::String("NW".to_string())],
        };
        let docs = saphyr::MarkedYaml::load_from_str("NW").unwrap();
        let value = docs.first().unwrap();
        let context = Context::default();
        let result = schema.validate(&context, value);
        assert!(result.is_ok());
    }
}
