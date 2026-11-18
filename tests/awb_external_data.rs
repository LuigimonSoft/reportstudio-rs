#[path = "../examples/awb/data_loader.rs"]
mod data_loader;

use reportstudio::{DataValue, ReportGenerator};
use serde_json::Value;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/awb")
        .join(name)
}

#[test]
fn template_is_empty_and_external_data_supplies_all_tables() {
    let template: Value =
        serde_json::from_slice(&std::fs::read(fixture("awb-template.json")).unwrap()).unwrap();
    assert!(template["datasets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|table| table["data"].as_array().unwrap().is_empty()));
    assert_eq!(template["relations"].as_array().unwrap().len(), 9);
    let data =
        data_loader::load_report_data(&fixture("awb-template.json"), &fixture("awb-data.json"))
            .unwrap();
    assert_eq!(data.tables().len(), 10);
    let goods = data
        .tables()
        .iter()
        .find(|table| table.name == "goods")
        .unwrap();
    assert_eq!(goods.rows().len(), 3);
    let awb_id = goods
        .columns()
        .iter()
        .position(|column| column.name == "awbId")
        .unwrap();
    assert_eq!(goods.rows()[0].values()[awb_id], DataValue::Decimal(1001.0));
}

#[test]
fn external_values_change_the_generated_pdf() {
    let dir = std::env::temp_dir().join(format!("reportstudio-awb-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("data.json");
    let mut source: Value =
        serde_json::from_slice(&std::fs::read(fixture("awb-data.json")).unwrap()).unwrap();
    let original =
        data_loader::load_report_data(&fixture("awb-template.json"), &fixture("awb-data.json"))
            .unwrap();
    let first = ReportGenerator::new(fixture("awb-template.json"))
        .with_data(original)
        .to_pdf_bytes()
        .unwrap();
    source["tables"]["shipper"][0]["name"] = "DIFFERENT SHIPPER".into();
    std::fs::write(&path, serde_json::to_vec(&source).unwrap()).unwrap();
    let changed = data_loader::load_report_data(&fixture("awb-template.json"), &path).unwrap();
    let second = ReportGenerator::new(fixture("awb-template.json"))
        .with_data(changed)
        .to_pdf_bytes()
        .unwrap();
    assert!(first.starts_with(b"%PDF"));
    assert!(second.starts_with(b"%PDF"));
    assert_ne!(first, second);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_missing_tables_and_invalid_values() {
    let dir = std::env::temp_dir().join(format!("reportstudio-awb-invalid-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("data.json");
    let mut source: Value =
        serde_json::from_slice(&std::fs::read(fixture("awb-data.json")).unwrap()).unwrap();
    source["tables"].as_object_mut().unwrap().remove("goods");
    std::fs::write(&path, serde_json::to_vec(&source).unwrap()).unwrap();
    assert!(
        data_loader::load_report_data(&fixture("awb-template.json"), &path)
            .unwrap_err()
            .to_string()
            .contains("goods")
    );
    source = serde_json::from_slice(&std::fs::read(fixture("awb-data.json")).unwrap()).unwrap();
    source["tables"]["goods"][0]["pieces"] = "not a number".into();
    std::fs::write(&path, serde_json::to_vec(&source).unwrap()).unwrap();
    assert!(data_loader::load_report_data(&fixture("awb-template.json"), &path).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
