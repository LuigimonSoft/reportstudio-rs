//! Apply saved report parameters to caller-supplied data without mutating it.
use crate::repository::{DataValue, Error, ReportData, Result};
use serde_json::Value;

pub(super) fn apply(project: &Value, data: &ReportData) -> Result<ReportData> {
    let Some(parameters) = project.get("parameters") else {
        return Ok(data.clone());
    };
    let parameters = parameters
        .as_array()
        .ok_or_else(|| Error::InvalidReport("parameters must be an array".into()))?;
    let mut filtered = ReportData::new();
    for parameter in parameters {
        let id = parameter["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::InvalidReport("parameter id is missing".into()))?;
        let value = parameter
            .get("value")
            .filter(|v| !v.is_null())
            .or_else(|| parameter.get("defaultValue"));
        let empty = value.is_none_or(|v| v.is_null() || v.as_str().is_some_and(str::is_empty));
        if empty && parameter["required"] == true {
            return Err(Error::InvalidData(format!("required parameter: {id}")));
        }
        if empty {
            continue;
        }
        let dataset = parameter["dataset"]
            .as_str()
            .ok_or_else(|| Error::InvalidReport(format!("parameter {id} dataset is missing")))?;
        let field = parameter["field"]
            .as_str()
            .ok_or_else(|| Error::InvalidReport(format!("parameter {id} field is missing")))?;
        let table = data.table(dataset).ok_or_else(|| {
            Error::InvalidData(format!("parameter {id}: missing table {dataset}"))
        })?;
        if !table.columns().iter().any(|c| c.name == field) {
            return Err(Error::InvalidData(format!(
                "parameter {id}: unknown field {dataset}.{field}"
            )));
        }
        if !matches!(
            parameter["operator"].as_str(),
            Some("eq" | "contains" | "gte" | "lte")
        ) {
            return Err(Error::Unsupported(format!("parameter {id} operator")));
        }
        if !value.unwrap().is_string()
            && !value.unwrap().is_number()
            && !value.unwrap().is_boolean()
        {
            return Err(Error::InvalidReport(format!(
                "invalid value for parameter {id}"
            )));
        }
    }
    for source in data.tables() {
        let mut table = crate::repository::DataTable::new(&source.name);
        for column in source.columns() {
            table.add_column(&column.name, column.data_type)?;
        }
        for row in source.rows() {
            let mut include = true;
            for p in parameters
                .iter()
                .filter(|p| p["dataset"].as_str() == Some(source.name.as_str()))
            {
                let value = p
                    .get("value")
                    .filter(|v| !v.is_null())
                    .or_else(|| p.get("defaultValue"));
                let Some(value) =
                    value.filter(|v| !v.is_null() && !v.as_str().is_some_and(str::is_empty))
                else {
                    continue;
                };
                let expected = value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string());
                let actual = source.field(row, p["field"].as_str().unwrap()).unwrap();
                let text = actual.display();
                let result = match p["operator"].as_str().unwrap() {
                    "eq" => text == expected,
                    "contains" => text.to_lowercase().contains(&expected.to_lowercase()),
                    "gte" | "lte" if !matches!(actual, DataValue::Null) => {
                        let ordering = match actual {
                            DataValue::Integer(n) => {
                                if let Ok(bound) = expected.parse::<i64>() {
                                    Some(n.cmp(&bound))
                                } else {
                                    let bound = expected.parse::<f64>().map_err(|_| {
                                        Error::InvalidData("parameter expects a number".into())
                                    })?;
                                    if !bound.is_finite() || n.unsigned_abs() > (1_u64 << 53) {
                                        return Err(Error::Unsupported(
                                            "numeric parameter precision".into(),
                                        ));
                                    }
                                    (*n as f64).partial_cmp(&bound)
                                }
                            }
                            DataValue::Decimal(n) => {
                                let bound = expected.parse::<f64>().map_err(|_| {
                                    Error::InvalidData("parameter expects a number".into())
                                })?;
                                if !bound.is_finite() {
                                    return Err(Error::InvalidData(
                                        "parameter expects a finite number".into(),
                                    ));
                                }
                                n.partial_cmp(&bound)
                            }
                            _ => Some(text.cmp(&expected)),
                        };
                        if p["operator"] == "gte" {
                            ordering.is_some_and(|o| o.is_ge())
                        } else {
                            ordering.is_some_and(|o| o.is_le())
                        }
                    }
                    _ => false,
                };
                if !result {
                    include = false;
                    break;
                }
            }
            if include {
                table.add_row(row.values().to_vec())?;
            }
        }
        filtered.add_table(table)?;
    }
    Ok(filtered)
}
