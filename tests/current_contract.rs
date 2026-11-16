mod support;
use reportstudio::{DataTable, DataType, DataValue, Error, ReportGenerator};
use serde_json::json;
use support::*;
#[test]
fn static_and_repeated_fields_are_in_the_generated_pdf() {
    let f = filled();
    let pdf = f.generate().unwrap();
    assert_pdf(&pdf);
    for value in ["Title", "Alpha", "Beta"] {
        assert!(has_text(&pdf, value), "{value}");
    }
}
#[test]
fn qualified_fields_resolve_with_their_table_id() {
    let mut f = filled();
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[details.name]");
    let bytes = f.generate().unwrap();
    assert!(has_text(&bytes, "Alpha"));
    assert!(has_text(&bytes, "Beta"));
}
#[test]
fn null_fields_become_empty_text() {
    let mut f = Fixture::new();
    let mut t = reportstudio::DataTable::new("details");
    t.add_column("name", DataType::Text).unwrap();
    t.add_row(vec![DataValue::Null]).unwrap();
    f.data.add_table(t).unwrap();
    assert_pdf(&f.generate().unwrap());
}
#[test]
fn missing_table_returns_a_typed_error() {
    assert!(matches!(
        Fixture::new().generate(),
        Err(Error::InvalidData(_))
    ));
}
#[test]
fn unknown_field_returns_a_typed_error() {
    let mut f = filled();
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[missing]");
    assert!(matches!(f.generate(), Err(Error::InvalidData(_))));
}
#[test]
fn malformed_expression_is_not_silently_output() {
    let mut f = filled();
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[name");
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn invalid_page_margins_are_rejected() {
    let mut f = filled();
    f.report["report"]["page"]["margins"]["left"] = json!(9);
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn negative_geometry_is_rejected() {
    let mut f = filled();
    f.report["report"]["header"]["elements"][0]["x"] = json!(-1);
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn unsupported_version_is_rejected() {
    let mut f = filled();
    f.report["schemaVersion"] = json!(999);
    assert!(matches!(f.generate(), Err(Error::Unsupported(_))));
}
#[test]
fn hidden_element_is_omitted() {
    let mut f = filled();
    f.report["report"]["header"]["elements"][0]["visible"] = json!(false);
    let bytes = f.generate().unwrap();
    assert!(!has_text(&bytes, "Title"));
    assert!(has_text(&bytes, "Alpha"));
}
#[test]
fn hidden_section_is_omitted() {
    let mut f = filled();
    f.report["report"]["header"]["visible"] = json!(false);
    let bytes = f.generate().unwrap();
    assert!(!has_text(&bytes, "Title"));
}
#[test]
fn pdf_special_characters_are_escaped() {
    let mut f = filled();
    f.report["report"]["header"]["elements"][0]["text"] = json!("(back\\slash)");
    let bytes = f.generate().unwrap();
    assert!(has_text(&bytes, r"\(back\\slash\)"));
}
#[test]
fn malformed_json_returns_a_json_error() {
    let f = filled();
    std::fs::write(f.report_path(), b"{broken").unwrap();
    assert!(matches!(
        ReportGenerator::new(f.report_path()).to_pdf_bytes(),
        Err(Error::Json(_))
    ));
}
#[test]
fn missing_file_returns_an_io_error() {
    let f = Fixture::new();
    assert!(matches!(
        ReportGenerator::new(f.report_path()).to_pdf_bytes(),
        Err(Error::Io(_))
    ));
}
#[test]
fn failed_generation_does_not_overwrite_existing_output() {
    let mut f = filled();
    f.report["report"]["header"]["elements"][0]["type"] = json!("chart");
    std::fs::write(f.output_path(), b"keep me").unwrap();
    assert!(f.save().is_err());
    assert_eq!(std::fs::read(f.output_path()).unwrap(), b"keep me");
}
#[test]
fn bytes_and_saved_file_are_identical() {
    let f = filled();
    let bytes = f.generate().unwrap();
    f.save().unwrap();
    assert_eq!(std::fs::read(f.output_path()).unwrap(), bytes);
}
#[test]
fn unsupported_expressions_do_not_execute_or_create_output() {
    let mut f = filled();
    f.report["report"]["formulas"] = json!([{"name":"bad","expression":"process.exit()"}]);
    assert!(matches!(f.save(), Err(Error::Unsupported(_))));
    assert!(!f.output_path().exists());
}
#[test]
fn oversized_element_is_rejected() {
    let mut f = filled();
    f.report["report"]["header"]["elements"][0]["height"] = json!(20);
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn unsupported_schema_version_does_not_overwrite_file() {
    let mut f = filled();
    f.report["schemaVersion"] = json!(2);
    std::fs::write(f.output_path(), b"existing").unwrap();
    assert!(matches!(f.save(), Err(Error::Unsupported(_))));
    assert_eq!(std::fs::read(f.output_path()).unwrap(), b"existing");
}

#[test]
fn large_integer_values_never_silently_lose_precision() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Integer).unwrap();
    t.add_row(vec![i64::MAX.into()]).unwrap();
    f.data.add_table(t).unwrap();
    f.report["datasets"][0]["schema"]["fields"][0]["type"] = json!("number");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "9223372036854775807"));
}
#[test]
fn valid_parameter_is_not_silently_ignored_before_filtering_exists() {
    let mut f = filled();
    f.report["parameters"] = json!([{"id":"name","type":"text","dataset":"details","field":"name","operator":"eq","value":"Alpha","required":true}]);
    assert!(
        matches!(f.generate(), Err(Error::Unsupported(feature)) if feature.contains("parameter"))
    );
}
#[test]
fn empty_message_does_not_block_populated_details() {
    let mut f = filled();
    f.report["report"]["details"][0]["emptyMessage"] = json!("No rows");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Alpha"));
    assert!(!has_text(&pdf, "No rows"));
}
#[test]
fn empty_message_is_not_silently_discarded_when_no_rows_exist() {
    let mut f = Fixture::new();
    f.data.add_table(table("details", &[])).unwrap();
    f.report["report"]["details"][0]["emptyMessage"] = json!("No rows");
    assert!(matches!(f.generate(), Err(Error::Unsupported(_))));
}
#[test]
fn winansi_accents_are_encoded_without_corrupting_the_pdf() {
    let mut f = filled();
    f.report["report"]["header"]["elements"][0]["text"] = json!("México · €");
    let bytes = f.generate().unwrap();
    assert_pdf(&bytes);
    assert!(has_text(&bytes, r"M\351xico \267 \200"));
}
#[test]
fn unsupported_writing_system_returns_an_error_instead_of_losing_text() {
    let mut f = filled();
    f.report["report"]["header"]["elements"][0]["text"] = json!("東京");
    assert!(matches!(f.generate(), Err(Error::Unsupported(_))));
}
