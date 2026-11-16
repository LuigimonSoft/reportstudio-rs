//! Validate supplied table definitions against the report's declared schemas.
use crate::repository::{DataType, Error, ReportData, Result};
use serde_json::Value;

pub(super) fn validate(project: &Value, data: &ReportData) -> Result<()> {
    let Some(datasets) = project.get("datasets") else {
        return Ok(());
    };
    let datasets = datasets
        .as_array()
        .ok_or_else(|| Error::InvalidReport("datasets must be an array".into()))?;
    for source in datasets {
        let id = source["id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| Error::InvalidReport("dataset id is missing".into()))?;
        let Some(table) = data.table(id) else {
            continue;
        };
        let fields = source["schema"]["fields"]
            .as_array()
            .ok_or_else(|| Error::InvalidReport(format!("missing fields for table {id}")))?;
        for field in fields {
            let name = field["name"]
                .as_str()
                .filter(|name| !name.is_empty())
                .ok_or_else(|| Error::InvalidReport(format!("missing field name in {id}")))?;
            let kind = field["type"].as_str().ok_or_else(|| {
                Error::InvalidReport(format!("missing field type for {id}.{name}"))
            })?;
            let column = table
                .columns()
                .iter()
                .find(|column| column.name == name)
                .ok_or_else(|| Error::InvalidData(format!("missing field {id}.{name}")))?;
            let valid = match kind {
                "text" => matches!(column.data_type, DataType::Text | DataType::Date),
                "number" => matches!(column.data_type, DataType::Integer | DataType::Decimal),
                "boolean" => column.data_type == DataType::Boolean,
                _ => {
                    return Err(Error::InvalidReport(format!(
                        "unsupported declared field type {kind} for {id}.{name}"
                    )))
                }
            };
            if !valid {
                return Err(Error::InvalidData(format!(
                    "wrong type for {id}.{name}: expected {kind}"
                )));
            }
        }
    }
    Ok(())
}
