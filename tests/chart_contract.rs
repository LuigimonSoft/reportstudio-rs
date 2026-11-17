mod support;
use reportstudio::{DataTable, DataType, Error};
use serde_json::json;
use support::*;
fn chart() -> Fixture {
    let mut f = Fixture::new();
    let mut table = DataTable::new("details");
    table.add_column("name", DataType::Text).unwrap();
    table.add_column("amount", DataType::Decimal).unwrap();
    for (name, n) in [("Alpha", 10.0), ("Beta", 5.0), ("Alpha", 15.0)] {
        table.add_row(vec![name.into(), n.into()]).unwrap();
    }
    f.data.add_table(table).unwrap();
    let mut graph = text("");
    graph["type"] = json!("chart");
    graph["y"] = json!(0.4);
    graph["width"] = json!(4);
    graph["height"] = json!(2);
    graph["chart"] = json!({"kind":"column","dataset":"details","category":"name","value":"amount","aggregation":"sum","sort":"descending","limit":10,"title":"Sales","color":"#2563eb","categoryColors":{"Alpha":"#ff0000"},"background":"#ffffff","showLabels":true,"showGrid":true,"showLegend":true});
    f.report["report"]["header"]["height"] = json!(2.5);
    f.report["report"]["header"]["elements"]
        .as_array_mut()
        .unwrap()
        .push(graph);
    f
}
#[test]
fn categories_are_aggregated_and_colors_are_rendered() {
    let pdf = chart().generate().unwrap();
    let source = String::from_utf8_lossy(&pdf);
    assert!(source.contains("(Sales)"));
    assert!(source.contains("(Alpha)"));
    assert!(source.contains("(Beta)"));
    assert!(source.contains("(25)"));
    assert!(source.contains("(5)"));
    assert!(source.contains("1.0000 0.0000 0.0000 rg"));
    assert!(source.contains("0.1451 0.3882 0.9216 rg"));
}
#[test]
fn non_supported_chart_kind_fails_instead_of_producing_blank_output() {
    let mut f = chart();
    f.report["report"]["header"]["elements"][1]["chart"]["kind"] = json!("pie");
    assert!(matches!(f.generate(), Err(Error::Unsupported(_))));
}
#[test]
fn chart_reads_supplied_records_and_validates_value_column() {
    let mut f = chart();
    f.report["report"]["header"]["elements"][1]["chart"]["value"] = json!("missing");
    assert!(matches!(f.generate(), Err(Error::InvalidData(_))));
}
