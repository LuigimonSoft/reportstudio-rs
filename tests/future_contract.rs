//! Acceptance contracts for features not implemented yet. Each ignored test
//! expresses a desired result; none accepts `Unsupported` as a success.
mod support;
use reportstudio::{DataTable, DataType};
use serde_json::json;
use support::*;

#[test]
#[ignore = "Unicode and embedded fonts are not implemented"]
fn unicode_accents_and_non_latin_characters_survive_pdf_generation() {
    let mut f = filled();
    f.report["report"]["header"]["elements"][0]["text"] = json!("México Montréal 東京");
    let pdf = f.generate().unwrap();
    assert_pdf(&pdf);
    assert!(
        has_text(&pdf, "México Montréal 東京"),
        "PDF text must be extractable with the original Unicode characters"
    );
}

#[test]
fn multiline_text_preserves_both_lines() {
    let mut f = filled();
    f.report["report"]["header"]["height"] = json!(0.8);
    f.report["report"]["header"]["elements"][0]["height"] = json!(0.6);
    f.report["report"]["header"]["elements"][0]["text"] = json!("First line\nSecond line");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "First line"));
    assert!(has_text(&pdf, "Second line"));
}

#[test]
#[ignore = "page and report footers are not implemented"]
fn footer_content_is_included() {
    let mut f = filled();
    f.report["report"]["footer"] =
        json!({"id":"footer","name":"Footer","height":0.4,"elements":[text("Footer note")]});
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Footer note"));
}

#[test]
#[ignore = "page number special fields are not implemented"]
fn page_number_and_count_are_evaluated() {
    let mut f = filled();
    f.report["report"]["footer"] =
        json!({"id":"footer","height":0.4,"elements":[text("Page [PAGE_NUMBER] of [PAGE_COUNT]")]});
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Page 1 of 1"));
    assert!(!has_text(&pdf, "PAGE_NUMBER"));
}

#[test]
#[ignore = "multi-page layout is not implemented"]
fn large_detail_sets_produce_multiple_pages_without_losing_rows() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Text).unwrap();
    for n in 0..60 {
        t.add_row(vec![format!("Item{n:02}").into()]).unwrap();
    }
    f.data.add_table(t).unwrap();
    let pdf = f.generate().unwrap();
    assert_pdf(&pdf);
    assert!(has_text(&pdf, "Item00"));
    assert!(has_text(&pdf, "Item59"));
    let s = String::from_utf8_lossy(&pdf);
    assert!(
        s.matches("/Type /Page ").count() >= 2,
        "expected multiple page objects"
    );
}

#[test]
#[ignore = "section visibility expressions are not implemented"]
fn boolean_section_formula_hides_content() {
    let mut f = filled();
    f.report["report"]["header"]["visibilityExpression"] = json!("false");
    let pdf = f.generate().unwrap();
    assert!(!has_text(&pdf, "Title"));
    assert!(has_text(&pdf, "Alpha"));
}

#[test]
#[ignore = "nested sections are not implemented"]
fn nested_section_content_is_rendered() {
    let mut f = filled();
    f.report["report"]["header"]["sections"] =
        json!([{"id":"child","name":"Child","height":0.3,"elements":[text("Child content")]}]);
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Child content"));
}

#[test]
#[ignore = "formula fields are not implemented"]
fn named_formula_is_evaluated_in_detail_context() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("quantity", DataType::Integer).unwrap();
    t.add_column("price", DataType::Decimal).unwrap();
    t.add_row(vec![2.into(), 10.into()]).unwrap();
    t.add_row(vec![3.into(), 10.into()]).unwrap();
    f.data.add_table(t).unwrap();
    f.report["report"]["formulas"] = json!([{"name":"LineTotal","expression":"quantity * price"}]);
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[LineTotal]");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "20"));
    assert!(has_text(&pdf, "30"));
}

#[test]
#[ignore = "SUM aggregate expressions are not implemented"]
fn sum_expression_uses_all_detail_rows() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("amount", DataType::Decimal).unwrap();
    t.add_row(vec![10.into()]).unwrap();
    t.add_row(vec![20.into()]).unwrap();
    f.data.add_table(t).unwrap();
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[amount]");
    f.report["report"]["details"][0]["groupFooter"] =
        json!({"id":"totals","height":0.3,"elements":[text("[SUM(amount)]")]});
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "30"));
    assert!(!has_text(&pdf, "SUM(amount)"));
}

#[test]
#[ignore = "number and currency formatting are not implemented"]
fn currency_values_follow_report_locale_and_currency() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("amount", DataType::Decimal).unwrap();
    t.add_row(vec![1500.25.into()]).unwrap();
    f.data.add_table(t).unwrap();
    let e = &mut f.report["report"]["details"][0]["band"]["elements"][0];
    e["text"] = json!("[amount]");
    e["style"]["format"] = json!("currency");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "$1,500.25"));
}

#[test]
fn detail_resolves_parent_field_through_relationship() {
    let mut f = Fixture::new();
    let mut parent = DataTable::new("customers");
    parent.add_column("id", DataType::Integer).unwrap();
    parent.add_column("name", DataType::Text).unwrap();
    parent
        .add_row(vec![1.into(), "Client Alpha".into()])
        .unwrap();
    let mut child = DataTable::new("details");
    child.add_column("customerId", DataType::Integer).unwrap();
    child.add_row(vec![1.into()]).unwrap();
    f.data.add_table(parent).unwrap();
    f.data.add_table(child).unwrap();
    f.report["datasets"][0]["schema"]["fields"] = json!([{"name":"customerId","type":"number"}]);
    f.report["relations"] = json!([{"id":"r1","from":{"dataset":"details","field":"customerId"},"to":{"dataset":"customers","field":"id"},"cardinality":"n:1"}]);
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[customers.name]");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Client Alpha"));
}

#[test]
fn related_detail_rows_do_not_include_other_parent_records() {
    let mut f = Fixture::new();
    let mut parent = DataTable::new("orders");
    parent.add_column("id", DataType::Integer).unwrap();
    parent.add_row(vec![1.into()]).unwrap();
    let mut child = DataTable::new("details");
    child.add_column("orderId", DataType::Integer).unwrap();
    child.add_column("name", DataType::Text).unwrap();
    child.add_row(vec![1.into(), "Included".into()]).unwrap();
    child.add_row(vec![2.into(), "Other order".into()]).unwrap();
    f.data.add_table(parent).unwrap();
    f.data.add_table(child).unwrap();
    f.report["dataModel"] = json!({"rootDataset":"orders","tables":{}});
    f.report["relations"] = json!([{"id":"r","from":{"dataset":"orders","field":"id"},"to":{"dataset":"details","field":"orderId"},"cardinality":"1:n"}]);
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Included"));
    assert!(!has_text(&pdf, "Other order"));
}

#[test]
fn relationship_to_missing_field_produces_data_error() {
    let mut f = filled();
    f.report["relations"] = json!([{"id":"r","from":{"dataset":"details","field":"bad"},"to":{"dataset":"details","field":"name"},"cardinality":"1:1"}]);
    assert!(matches!(
        f.generate(),
        Err(reportstudio::Error::InvalidData(_)) | Err(reportstudio::Error::InvalidReport(_))
    ));
}

#[test]
fn required_parameter_without_value_fails_validation() {
    let mut f = filled();
    f.report["parameters"] = json!([{"id":"start","label":"Start date","type":"date","dataset":"details","field":"name","operator":"gte","required":true}]);
    assert!(matches!(
        f.generate(),
        Err(reportstudio::Error::InvalidData(_)) | Err(reportstudio::Error::InvalidReport(_))
    ));
}

#[test]
fn line_and_rectangle_are_rendered_as_graphics() {
    let mut f = filled();
    f.report["report"]["header"]["elements"].as_array_mut().unwrap().push(json!({"id":"box","type":"box","x":0,"y":0,"width":3,"height":0.2,"text":"","src":"","style":{"fontsize":9,"color":"#000000"}}));
    let pdf = f.generate().unwrap();
    assert_pdf(&pdf);
    assert!(
        String::from_utf8_lossy(&pdf).contains(" re"),
        "expected PDF rectangle drawing operator"
    );
}

#[test]
#[ignore = "charts are not implemented"]
fn bound_chart_renders_without_inserting_error_text() {
    let mut f = filled();
    let mut chart = text("");
    chart["type"] = json!("chart");
    chart["chart"] = json!({"kind":"bar","dataset":"details","category":"name","value":"name","aggregation":"count","sort":"source","limit":10,"title":"Sales chart","color":"#2563eb","background":"#ffffff","showLegend":false,"showLabels":true,"showGrid":true});
    f.report["report"]["header"]["elements"]
        .as_array_mut()
        .unwrap()
        .push(chart);
    let pdf = f.generate().unwrap();
    assert_pdf(&pdf);
    assert!(!has_text(&pdf, "No chart data"));
}

#[test]
fn incompatible_supplied_column_type_is_rejected() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Integer).unwrap();
    t.add_row(vec![10.into()]).unwrap();
    f.data.add_table(t).unwrap();
    assert!(matches!(
        f.generate(),
        Err(reportstudio::Error::InvalidData(_))
    ));
}

#[test]
fn missing_declared_column_is_rejected_even_with_no_rows() {
    let mut f = Fixture::new();
    f.data.add_table(DataTable::new("details")).unwrap();
    assert!(matches!(
        f.generate(),
        Err(reportstudio::Error::InvalidData(_))
    ));
}

#[test]
fn table_without_columns_cannot_accept_a_row() {
    let mut t = DataTable::new("empty");
    assert!(t.add_row(vec![]).is_err());
}

#[test]
#[ignore = "AWB rendering, numeric formatting, expressions and footer placement are incomplete"]
fn awb_renders_reference_content_and_all_three_goods_rows() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/awb/awb-full-form-example.json"
    );
    let project: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let data = sample_data(&project);
    let pdf = reportstudio::ReportGenerator::new(path)
        .with_data(data)
        .to_pdf_bytes()
        .unwrap();
    assert_pdf(&pdf);
    for value in [
        "999-12345675",
        "NORTHSTAR EXPORTS LLC",
        "DEMO IMPORTS SA",
        "ELECTRONIC EQUIPMENT",
        "MACHINE PARTS",
        "PRINTED MATERIAL",
    ] {
        assert!(has_text(&pdf, value), "missing AWB data: {value}");
    }
}

#[test]
#[ignore = "AWB rendering and external data substitution are not implemented"]
fn awb_uses_supplied_records_instead_of_embedded_fixture_records() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/awb/awb-full-form-example.json"
    );
    let mut project: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let shipper = project["datasets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "shipper")
        .unwrap();
    shipper["data"][0]["name"] = json!("REPLACED SHIPPER");
    let data = sample_data(&project);
    let pdf = reportstudio::ReportGenerator::new(path)
        .with_data(data)
        .to_pdf_bytes()
        .unwrap();
    assert!(has_text(&pdf, "REPLACED SHIPPER"));
    assert!(!has_text(&pdf, "NORTHSTAR EXPORTS LLC"));
}

#[test]
fn repeated_detail_table_renders_every_record() {
    let mut f = Fixture::new();
    f.report = serde_json::from_str(include_str!("browser/fixtures/table-report.json")).unwrap();
    let mut details = DataTable::new("details");
    details.add_column("description", DataType::Text).unwrap();
    details.add_row(vec!["First item".into()]).unwrap();
    details.add_row(vec!["Second item".into()]).unwrap();
    f.data.add_table(details).unwrap();
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "First item"));
    assert!(has_text(&pdf, "Second item"));
}

#[test]
#[ignore = "page footer repetition is not implemented"]
fn footer_appears_on_every_page_of_a_long_report() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Text).unwrap();
    for i in 0..60 {
        t.add_row(vec![format!("Line{i}").into()]).unwrap();
    }
    f.data.add_table(t).unwrap();
    f.report["report"]["pageFooter"] =
        json!({"id":"pf","height":0.4,"elements":[text("Page footer marker")]});
    let pdf = f.generate().unwrap();
    let source = String::from_utf8_lossy(&pdf);
    assert!(source.matches("Page footer marker").count() >= 2);
}

#[test]
#[ignore = "ROW_NUMBER special field is not implemented"]
fn row_number_is_evaluated_for_each_detail_record() {
    let mut f = filled();
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] =
        json!("Row [ROW_NUMBER]: [name]");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Row 1: Alpha"));
    assert!(has_text(&pdf, "Row 2: Beta"));
}

#[test]
#[ignore = "formatted section colors are not implemented"]
fn fixed_section_background_is_rendered() {
    let mut f = filled();
    f.report["report"]["header"]["background"] = json!("#ff0000");
    let pdf = f.generate().unwrap();
    assert_pdf(&pdf);
    let s = String::from_utf8_lossy(&pdf);
    assert!(
        s.contains(" re"),
        "section background must draw a rectangle"
    );
}

#[test]
#[ignore = "expression errors are not implemented"]
fn circular_formulas_fail_with_a_clear_error() {
    let mut f = filled();
    f.report["report"]["formulas"] =
        json!([{"name":"A","expression":"B + 1"},{"name":"B","expression":"A + 1"}]);
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[A]");
    assert!(matches!(
        f.generate(),
        Err(reportstudio::Error::InvalidReport(_))
    ));
}

#[test]
#[ignore = "images are not implemented"]
fn embedded_image_produces_a_pdf_image_object() {
    let mut f = filled();
    let mut image = text("");
    image["type"] = json!("image");
    image["src"] = json!("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jCYQAAAAASUVORK5CYII=");
    f.report["report"]["header"]["elements"]
        .as_array_mut()
        .unwrap()
        .push(image);
    let pdf = f.generate().unwrap();
    assert_pdf(&pdf);
    assert!(
        has_text(&pdf, "/Image"),
        "embedded picture must be represented in PDF resources"
    );
}

#[test]
#[ignore = "report parameters and filtering are not implemented"]
fn saved_parameter_filters_detail_rows() {
    let mut f = filled();
    f.report["parameters"] = json!([{"id":"filter","label":"Name","type":"text","dataset":"details","field":"name","operator":"eq","value":"Alpha","saveValue":true}]);
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Alpha"));
    assert!(!has_text(&pdf, "Beta"));
}

#[test]
#[ignore = "per-row section visibility is not implemented"]
fn section_visibility_formula_filters_rows_by_boolean_field() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Text).unwrap();
    t.add_column("enabled", DataType::Boolean).unwrap();
    t.add_row(vec!["Visible".into(), true.into()]).unwrap();
    t.add_row(vec!["Hidden".into(), false.into()]).unwrap();
    f.data.add_table(t).unwrap();
    f.report["report"]["details"][0]["band"]["visibilityExpression"] = json!("enabled");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Visible"));
    assert!(!has_text(&pdf, "Hidden"));
}

#[test]
#[ignore = "grouped details are not implemented"]
fn grouping_emits_a_header_for_each_distinct_group() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Text).unwrap();
    t.add_column("category", DataType::Text).unwrap();
    t.add_row(vec!["First".into(), "North".into()]).unwrap();
    t.add_row(vec!["Second".into(), "South".into()]).unwrap();
    f.data.add_table(t).unwrap();
    f.report["report"]["details"][0]["groupBy"] = json!("category");
    f.report["report"]["details"][0]["groupHeader"] =
        json!({"id":"gh","height":0.3,"elements":[text("Group: [category]")]});
    let pdf = f.generate().unwrap();
    for value in ["Group: North", "Group: South", "First", "Second"] {
        assert!(has_text(&pdf, value));
    }
}

#[test]
#[ignore = "conditional section backgrounds are not implemented"]
fn formula_can_select_a_section_background_color() {
    let mut f = filled();
    f.report["report"]["header"]["backgroundExpression"] = json!("IF(1 = 1, '#ff0000', '#0000ff')");
    let pdf = f.generate().unwrap();
    assert_pdf(&pdf);
    assert!(
        has_text(&pdf, " re"),
        "background must draw a filled rectangle"
    );
}
