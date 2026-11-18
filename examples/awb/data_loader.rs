//! Example adapter: map an external JSON file into the library's typed tables.
use reportstudio::{DataTable, DataType, DataValue, ReportData};
use serde_json::Value;
use std::{error::Error, io, path::Path};

type ExampleResult<T> = Result<T, Box<dyn Error>>;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// Table IDs and field types come from the report; row values come only from data_path.
pub fn load_report_data(report_path: &Path, data_path: &Path) -> ExampleResult<ReportData> {
    let report: Value = serde_json::from_slice(&std::fs::read(report_path)?)?;
    let source: Value = serde_json::from_slice(&std::fs::read(data_path)?)?;
    let datasets = report["datasets"]
        .as_array()
        .ok_or_else(|| invalid("Missing report datasets"))?;
    let tables = source["tables"]
        .as_object()
        .ok_or_else(|| invalid("Expected a tables object in the data file"))?;
    for id in tables.keys() {
        if !datasets
            .iter()
            .any(|dataset| dataset["id"].as_str() == Some(id))
        {
            return Err(invalid(format!("Unknown table: {id}")).into());
        }
    }
    let mut report_data = ReportData::new();
    for dataset in datasets {
        let id = dataset["id"]
            .as_str()
            .ok_or_else(|| invalid("Missing dataset ID"))?;
        let fields = dataset["schema"]["fields"]
            .as_array()
            .ok_or_else(|| invalid(format!("Missing schema for {id}")))?;
        let rows = tables
            .get(id)
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(format!("Missing rows array for table {id}")))?;
        let mut table = DataTable::new(id);
        for field in fields {
            let name = field["name"]
                .as_str()
                .ok_or_else(|| invalid(format!("Missing field name in {id}")))?;
            let kind = match field["type"].as_str() {
                Some("text") => DataType::Text,
                Some("number") => DataType::Decimal,
                Some("boolean") => DataType::Boolean,
                other => {
                    return Err(
                        invalid(format!("Unsupported type {other:?} in {id}.{name}")).into(),
                    )
                }
            };
            table.add_column(name, kind)?;
        }
        for (index, row) in rows.iter().enumerate() {
            let object = row
                .as_object()
                .ok_or_else(|| invalid(format!("Expected an object in {id} row {}", index + 1)))?;
            for key in object.keys() {
                if !fields
                    .iter()
                    .any(|field| field["name"].as_str() == Some(key))
                {
                    return Err(invalid(format!("Unknown field {id}.{key}")).into());
                }
            }
            // DataTable expects values in the same order as the columns, regardless of JSON key order.
            let values = fields
                .iter()
                .map(|field| {
                    let name = field["name"].as_str().expect("validated field name");
                    match object.get(name) {
                        None | Some(Value::Null) => Ok(DataValue::Null),
                        Some(Value::String(value)) => Ok(DataValue::Text(value.clone())),
                        Some(Value::Bool(value)) => Ok(DataValue::Boolean(*value)),
                        Some(Value::Number(value)) => value
                            .as_f64()
                            .map(DataValue::Decimal)
                            .ok_or_else(|| invalid(format!("Invalid number in {id}.{name}"))),
                        _ => Err(invalid(format!("Unsupported value in {id}.{name}"))),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            table.add_row(values)?;
        }
        report_data.add_table(table)?;
    }
    Ok(report_data)
}
