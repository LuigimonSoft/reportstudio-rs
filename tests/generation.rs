use reportstudio::{DataTable, DataType, DataValue, Error, ReportData, ReportGenerator};
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "reportstudio-{}-{}-{name}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn report() -> serde_json::Value {
    serde_json::json!({"version":1,"schemaVersion":1,"datasets":[{"id":"details","schema":{"fields":[{"name":"description","type":"text"}]},"data":[]}],"report":{"page":{"width":8.5,"height":11,"margins":{"top":0.5,"bottom":0.5,"left":0.5,"right":0.5}},"header":{"height":0.5,"elements":[{"type":"text","x":0,"y":0,"width":3,"height":0.3,"text":"Invoice","style":{"fontsize":12,"bold":true}}]},"details":[{"dataset":"details","band":{"height":0.3,"elements":[{"type":"text","x":0,"y":0,"width":3,"height":0.2,"text":"[description]","style":{"fontsize":9}}]}}]}})
}
#[test]
fn creates_pdf_from_report_and_supplied_rows() {
    let json = temp_path("report.json");
    let pdf = temp_path("out.pdf");
    fs::write(&json, report().to_string()).unwrap();
    let mut table = DataTable::new("details");
    table.add_column("description", DataType::Text).unwrap();
    table.add_row(vec!["Item (one)".into()]).unwrap();
    table.add_row(vec!["Item two".into()]).unwrap();
    let mut data = ReportData::new();
    data.add_table(table).unwrap();
    let generator = ReportGenerator::new(&json).with_data(data);
    let bytes = generator.to_pdf_bytes().unwrap();
    assert!(bytes.starts_with(b"%PDF-1.4"));
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("Invoice"));
    assert!(text.contains("Item \\(one\\)"));
    assert!(text.contains("Item two"));
    generator.save_pdf(&pdf).unwrap();
    assert_eq!(fs::read(&pdf).unwrap(), bytes);
    fs::remove_file(json).unwrap();
    fs::remove_file(pdf).unwrap();
}
#[test]
fn rejects_bad_rows_duplicates_and_dates() {
    let mut table = DataTable::new("details");
    table.add_column("n", DataType::Integer).unwrap();
    table.add_column("date", DataType::Date).unwrap();
    assert!(table.add_row(vec![1.into()]).is_err());
    assert!(table
        .add_row(vec!["bad".into(), DataValue::Date("2024-01-01".into())])
        .is_err());
    assert!(table
        .add_row(vec![1.into(), DataValue::Date("2023-02-29".into())])
        .is_err());
    table
        .add_row(vec![1.into(), DataValue::Date("2024-02-29".into())])
        .unwrap();
    assert!(table.add_column("extra", DataType::Text).is_err());
    let mut data = ReportData::new();
    data.add_table(table.clone()).unwrap();
    assert!(data.add_table(table).is_err());
}
#[test]
fn unsupported_features_fail_without_writing_a_partial_pdf() {
    let json = temp_path("report.json");
    let pdf = temp_path("out.pdf");
    let mut value = report();
    value["report"]["header"]["elements"][0]["type"] = "chart".into();
    fs::write(&json, value.to_string()).unwrap();
    assert!(matches!(
        ReportGenerator::new(&json).save_pdf(&pdf),
        Err(Error::Unsupported(_))
    ));
    assert!(!pdf.exists());
    fs::remove_file(json).unwrap();
}
#[test]
fn missing_table_is_reported() {
    let json = temp_path("report.json");
    fs::write(&json, report().to_string()).unwrap();
    assert!(matches!(
        ReportGenerator::new(&json).to_pdf_bytes(),
        Err(Error::InvalidData(_))
    ));
    fs::remove_file(json).unwrap();
}
