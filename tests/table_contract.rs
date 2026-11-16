mod support;
use reportstudio::{DataTable, DataType, Error};
use serde_json::{json, Value};
use support::*;
fn table_fixture() -> Fixture {
    let mut f = Fixture::new();
    f.report = serde_json::from_str(include_str!("browser/fixtures/table-report.json")).unwrap();
    let mut details = DataTable::new("details");
    details.add_column("description", DataType::Text).unwrap();
    details.add_row(vec!["First item".into()]).unwrap();
    details.add_row(vec!["Second item".into()]).unwrap();
    f.data.add_table(details).unwrap();
    f
}
#[test]
fn missing_table_group_id_is_rejected() {
    let mut f = table_fixture();
    f.report["report"]["details"][0]["band"]["elements"][0]
        .as_object_mut()
        .unwrap()
        .remove("groupId");
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn table_cell_outside_declared_grid_is_rejected() {
    let mut f = table_fixture();
    f.report["report"]["details"][0]["band"]["elements"][0]["cell"]["row"] = json!(5);
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn inconsistent_table_settings_are_rejected() {
    let mut f = table_fixture();
    f.report["report"]["details"][0]["band"]["elements"][1]["table"]["width"] = json!(2.0);
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn conditional_background_is_not_silently_ignored() {
    let mut f = table_fixture();
    for el in f.report["report"]["details"][0]["band"]["elements"]
        .as_array_mut()
        .unwrap()
    {
        el["table"]["styles"] =
            json!({"row:0":{"backgroundExpression":"IF(ROW_NUMBER=1, '#cccccc', '#ffffff')"}});
    }
    assert!(matches!(f.generate(), Err(Error::Unsupported(_))));
}
#[test]
fn unsupported_unexpanded_table_is_an_error() {
    let mut f = table_fixture();
    let element = &mut f.report["report"]["details"][0]["band"]["elements"][0];
    element.as_object_mut().unwrap().remove("cell");
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn changing_values_changes_the_rendered_detail_rows() {
    let mut f = table_fixture();
    let mut table = DataTable::new("details");
    table.add_column("description", DataType::Text).unwrap();
    table.add_row(vec!["Replacement".into()]).unwrap();
    f.data = reportstudio::ReportData::new();
    f.data.add_table(table).unwrap();
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Replacement"));
    assert!(!has_text(&pdf, "First item"));
}
#[test]
fn table_fixture_has_expected_designer_representation() {
    let f = table_fixture();
    let elements = f.report["report"]["details"][0]["band"]["elements"]
        .as_array()
        .unwrap();
    assert_eq!(elements.len(), 2);
    assert!(elements.iter().all(|e| e["table"].is_object()
        && e["cell"].is_object()
        && e["groupId"] == Value::String("table-reference".into())));
}
