//! Generate the fictional AWB fixture with explicitly supplied table records.
use reportstudio::{DataTable, DataType, DataValue, ReportData, ReportGenerator};
use serde_json::Value;
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let destination = PathBuf::from(args.next().ok_or("expected output path")?);
    let shipper_override = args.next().and_then(|value| value.into_string().ok());
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/awb/awb-full-form-example.json");
    let project: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    let mut data = ReportData::new();
    for source in project["datasets"].as_array().ok_or("missing datasets")? {
        let id = source["id"].as_str().ok_or("missing table id")?;
        let fields = source["schema"]["fields"]
            .as_array()
            .ok_or("missing schema")?;
        let mut table = DataTable::new(id);
        for column in fields {
            let kind = match column["type"].as_str() {
                Some("text") => DataType::Text,
                Some("number") => DataType::Decimal,
                Some("boolean") => DataType::Boolean,
                _ => return Err("unsupported fixture type".into()),
            };
            table.add_column(column["name"].as_str().ok_or("missing field name")?, kind)?;
        }
        for record in source["data"].as_array().ok_or("missing records")? {
            let values = fields
                .iter()
                .map(|field| {
                    let name = field["name"].as_str().unwrap();
                    if id == "shipper" && name == "name" {
                        if let Some(value) = &shipper_override {
                            return Ok(DataValue::Text(value.clone()));
                        }
                    }
                    match record.get(name) {
                        None | Some(Value::Null) => Ok(DataValue::Null),
                        Some(Value::String(value)) => Ok(DataValue::Text(value.clone())),
                        Some(Value::Bool(value)) => Ok(DataValue::Boolean(*value)),
                        Some(Value::Number(value)) => value
                            .as_f64()
                            .map(DataValue::Decimal)
                            .ok_or("invalid numeric value"),
                        _ => Err("invalid record value"),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            table.add_row(values)?;
        }
        data.add_table(table)?;
    }
    ReportGenerator::new(path)
        .with_data(data)
        .save_pdf(destination)?;
    Ok(())
}
