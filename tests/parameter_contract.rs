mod support;
use serde_json::json;
use support::*;

#[test]
fn multiple_parameters_filter_without_mutating_caller_data() {
    let mut f = filled();
    f.report["parameters"] = json!([
        {"id":"name","type":"text","dataset":"details","field":"name","operator":"contains","value":"ALP"},
        {"id":"minimum","type":"text","dataset":"details","field":"name","operator":"gte","value":"A"}
    ]);
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Alpha"));
    assert!(!has_text(&pdf, "Beta"));
    assert_eq!(f.data.tables()[0].rows().len(), 2);
}
#[test]
fn invalid_filter_field_and_operator_are_rejected() {
    let mut f = filled();
    f.report["parameters"] =
        json!([{"id":"bad","dataset":"details","field":"unknown","operator":"eq","value":"x"}]);
    assert!(matches!(
        f.generate(),
        Err(reportstudio::Error::InvalidData(_))
    ));
    f.report["parameters"][0]["field"] = json!("name");
    f.report["parameters"][0]["operator"] = json!("invalid");
    assert!(matches!(
        f.generate(),
        Err(reportstudio::Error::Unsupported(_))
    ));
}
#[test]
fn numeric_bounds_and_default_values_are_applied() {
    let mut f = Fixture::new();
    let mut table = reportstudio::DataTable::new("details");
    table
        .add_column("name", reportstudio::DataType::Integer)
        .unwrap();
    for n in [2, 10, 20] {
        table.add_row(vec![n.into()]).unwrap();
    }
    f.data.add_table(table).unwrap();
    f.report["datasets"][0]["schema"]["fields"][0]["type"] = json!("number");
    f.report["parameters"] = json!([
        {"id":"min","dataset":"details","field":"name","operator":"gte","defaultValue":"10"},
        {"id":"max","dataset":"details","field":"name","operator":"lte","value":"10"}
    ]);
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "(10)"));
    assert!(!has_text(&pdf, "(2)"));
    assert!(!has_text(&pdf, "(20)"));
}
