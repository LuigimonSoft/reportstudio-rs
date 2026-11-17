mod support;
use reportstudio::{DataTable, DataType, Error, ReportData, ReportDefinition, ReportGenerator};
use serde::Serialize;
use serde_json::{json, Value};
use support::*;

fn rows() -> ReportData {
    let mut table = DataTable::new("details");
    table.add_column("name", DataType::Text).unwrap();
    table.add_row(vec!["Alpha".into()]).unwrap();
    table.add_row(vec!["Beta".into()]).unwrap();
    let mut data = ReportData::new();
    data.add_table(table).unwrap();
    data
}
#[test]
fn file_and_in_memory_model_produce_the_same_pdf() {
    let f = filled();
    let file_pdf = f.generate().unwrap();
    let memory = ReportDefinition::from_json_value(f.report.clone());
    let generator = ReportGenerator::from_definition(memory).with_data(rows());
    assert_eq!(generator.to_pdf_bytes().unwrap(), file_pdf);
    assert_eq!(generator.to_pdf_bytes().unwrap(), file_pdf);
    generator.save_pdf(f.output_path()).unwrap();
    assert_eq!(std::fs::read(f.output_path()).unwrap(), file_pdf);
}
#[derive(Serialize)]
struct ApplicationReport {
    version: u32,
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    report: Value,
    datasets: Value,
    extra: Value,
}
#[test]
fn application_serializable_model_keeps_the_report_json_and_generates_pdf() {
    let mut doc = basic_report();
    doc["extra"] = json!({"owner":"Example","futureField":[1,true]});
    let model = ApplicationReport {
        version: 1,
        schema_version: 1,
        report: doc["report"].clone(),
        datasets: doc["datasets"].clone(),
        extra: doc["extra"].clone(),
    };
    let definition = ReportDefinition::from_serializable(&model).unwrap();
    assert_eq!(definition.as_json_value(), &doc);
    let roundtrip = serde_json::to_string(&definition).unwrap();
    let deserialized: ReportDefinition = serde_json::from_str(&roundtrip).unwrap();
    assert_eq!(deserialized.as_json_value(), &doc);
    let pdf = ReportGenerator::from_definition(deserialized)
        .with_data(rows())
        .to_pdf_bytes()
        .unwrap();
    assert!(has_text(&pdf, "Alpha"));
    assert!(has_text(&pdf, "Beta"));
}
#[test]
fn json_text_is_parsed_in_memory_and_malformed_json_is_rejected() {
    let doc = basic_report().to_string();
    let definition = ReportDefinition::from_json_str(&doc).unwrap();
    assert!(ReportGenerator::from_definition(definition)
        .with_data(rows())
        .to_pdf_bytes()
        .is_ok());
    assert!(matches!(
        ReportDefinition::from_json_str("{bad json"),
        Err(Error::Json(_))
    ));
}
#[test]
fn invalid_model_returns_same_validation_error_and_does_not_overwrite_output() {
    let f = filled();
    let mut doc = basic_report();
    doc["version"] = json!(42);
    let generator =
        ReportGenerator::from_definition(ReportDefinition::from_json_value(doc)).with_data(rows());
    std::fs::write(f.output_path(), b"existing").unwrap();
    assert!(matches!(
        generator.save_pdf(f.output_path()),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(std::fs::read(f.output_path()).unwrap(), b"existing");
}
#[test]
fn required_data_and_replacement_data_are_enforced_for_memory_reports() {
    let definition = ReportDefinition::from_json_value(basic_report());
    assert!(matches!(
        ReportGenerator::from_definition(definition.clone()).to_pdf_bytes(),
        Err(Error::InvalidData(_))
    ));
    let mut replacement = DataTable::new("details");
    replacement.add_column("name", DataType::Text).unwrap();
    replacement.add_row(vec!["New value".into()]).unwrap();
    let mut new_data = ReportData::new();
    new_data.add_table(replacement).unwrap();
    let pdf = ReportGenerator::from_definition(definition)
        .with_data(rows())
        .with_data(new_data)
        .to_pdf_bytes()
        .unwrap();
    assert!(has_text(&pdf, "New value"));
    assert!(!has_text(&pdf, "Alpha"));
}
#[test]
fn serializable_model_failures_return_json_error() {
    struct Invalid;
    impl Serialize for Invalid {
        fn serialize<S: serde::Serializer>(
            &self,
            _serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("serialization failed"))
        }
    }
    assert!(matches!(
        ReportDefinition::from_serializable(&Invalid),
        Err(Error::Json(_))
    ));
}

#[test]
fn typed_project_can_be_edited_and_rendered_without_a_report_file() {
    let mut model: reportstudio::ReportProject = serde_json::from_value(basic_report()).unwrap();
    model.report.header.as_mut().unwrap().elements[0].text =
        Some("Changed from typed model".into());
    model.report.header.as_mut().unwrap().elements[0]
        .extra
        .insert("futureSetting".into(), json!({"enabled":true}));
    let json = serde_json::to_value(&model).unwrap();
    assert_eq!(
        json["report"]["header"]["elements"][0]["futureSetting"]["enabled"],
        true
    );
    let pdf = ReportGenerator::from_model(&model)
        .unwrap()
        .with_data(rows())
        .to_pdf_bytes()
        .unwrap();
    assert!(has_text(&pdf, "Changed from typed model"));
    assert!(has_text(&pdf, "Alpha"));
}
#[test]
fn typed_air_waybill_model_preserves_rendered_output_and_advanced_settings() {
    let original: Value =
        serde_json::from_str(include_str!("../examples/awb/awb-full-form-example.json")).unwrap();
    let model: reportstudio::ReportProject = serde_json::from_value(original.clone()).unwrap();
    let data = sample_data(&original);
    let direct = ReportGenerator::from_definition(ReportDefinition::from_json_value(original))
        .with_data(data.clone())
        .to_pdf_bytes()
        .unwrap();
    let typed = ReportGenerator::from_model(&model)
        .unwrap()
        .with_data(data)
        .to_pdf_bytes()
        .unwrap();
    assert_eq!(typed, direct);
}
#[test]
fn typed_model_rejects_wrong_field_types_and_generation_validates_required_content() {
    let mut doc = basic_report();
    doc["report"]["page"]["width"] = json!("wide");
    assert!(serde_json::from_value::<reportstudio::ReportProject>(doc).is_err());
    let model = reportstudio::ReportProject::default();
    assert!(ReportGenerator::from_model(&model)
        .unwrap()
        .to_pdf_bytes()
        .is_err());
}
