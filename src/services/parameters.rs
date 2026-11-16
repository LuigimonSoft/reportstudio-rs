//! Parameter validation. Filtering and external parameter overrides are pending.
use crate::repository::{Error, Result};
use serde_json::Value;

pub(super) fn validate(project: &Value) -> Result<()> {
    let Some(value) = project.get("parameters") else {
        return Ok(());
    };
    let parameters = value
        .as_array()
        .ok_or_else(|| Error::InvalidReport("parameters must be an array".into()))?;
    for parameter in parameters {
        let id = parameter["id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Error::InvalidReport("parameter id is missing".into()))?;
        if parameter["required"] == true {
            let present = [parameter.get("value"), parameter.get("defaultValue")]
                .into_iter()
                .flatten()
                .any(|value| match value {
                    Value::Null => false,
                    Value::String(text) => !text.trim().is_empty(),
                    Value::Number(_) | Value::Bool(_) => true,
                    _ => false,
                });
            if !present {
                return Err(Error::InvalidData(format!("required parameter: {id}")));
            }
        }
    }
    if parameters.is_empty() {
        Ok(())
    } else {
        Err(Error::Unsupported("report parameter filtering".into()))
    }
}
