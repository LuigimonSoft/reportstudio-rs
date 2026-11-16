use reportstudio::{DataTable, DataType, DataValue, ReportData};
use serde_json::Value;

#[test]
fn awb_example_loads_all_ten_sources_and_three_goods_rows() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/awb/awb-full-form-example.json"
    );
    let project: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut data = ReportData::new();
    for source in project["datasets"].as_array().unwrap() {
        let mut table = DataTable::new(source["id"].as_str().unwrap());
        let fields = source["schema"]["fields"].as_array().unwrap();
        for field in fields {
            let kind = match field["type"].as_str().unwrap() {
                "text" => DataType::Text,
                "number" => DataType::Decimal,
                "boolean" => DataType::Boolean,
                other => panic!("unexpected field type {other}"),
            };
            table
                .add_column(field["name"].as_str().unwrap(), kind)
                .unwrap();
        }
        for row in source["data"].as_array().unwrap() {
            let values = fields
                .iter()
                .map(|field| {
                    let key = field["name"].as_str().unwrap();
                    match row.get(key) {
                        None | Some(Value::Null) => DataValue::Null,
                        Some(Value::String(value)) => DataValue::Text(value.clone()),
                        Some(Value::Bool(value)) => DataValue::Boolean(*value),
                        Some(Value::Number(value)) => DataValue::Decimal(value.as_f64().unwrap()),
                        other => panic!("unsupported value {key}: {other:?}"),
                    }
                })
                .collect();
            table.add_row(values).unwrap();
        }
        data.add_table(table).unwrap();
    }
    assert_eq!(data.tables().len(), 10);
    assert_eq!(
        data.tables()
            .iter()
            .find(|table| table.name == "goods")
            .unwrap()
            .rows()
            .len(),
        3
    );
    assert_eq!(project["relations"].as_array().unwrap().len(), 9);
}
