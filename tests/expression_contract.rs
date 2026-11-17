mod support;
use reportstudio::{DataTable, DataType, Error};
use serde_json::json;
use support::*;

fn quantities() -> Fixture {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("quantity", DataType::Integer).unwrap();
    t.add_column("price", DataType::Decimal).unwrap();
    t.add_row(vec![2.into(), 10.into()]).unwrap();
    t.add_row(vec![3.into(), 10.into()]).unwrap();
    f.data.add_table(t).unwrap();
    f.report["datasets"][0]["schema"]["fields"] =
        json!([{"name":"quantity","type":"number"},{"name":"price","type":"number"}]);
    f
}
#[test]
fn named_formula_per_row_and_total_use_supplied_records() {
    let mut f = quantities();
    f.report["report"]["formulas"] = json!([{"name":"LineTotal","expression":"quantity * price"}]);
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[LineTotal]");
    f.report["report"]["details"][0]["groupFooter"] =
        json!({"id":"total","height":0.3,"elements":[text("Total [SUM(LineTotal)]")]});
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "(20)"));
    assert!(has_text(&pdf, "(30)"));
    assert!(has_text(&pdf, "(Total 50)"));
}
#[test]
fn conditional_background_and_visibility_follow_each_row() {
    let mut f = quantities();
    let band = &mut f.report["report"]["details"][0]["band"];
    band["visibilityExpression"] = json!("quantity >= 3");
    band["backgroundExpression"] = json!("IF(quantity = 3, '#ff0000', '#0000ff')");
    band["elements"][0]["text"] = json!("[quantity]");
    let pdf = f.generate().unwrap();
    assert!(!has_text(&pdf, "(2)"));
    assert!(has_text(&pdf, "(3)"));
    assert!(has_text(&pdf, "1.0000 0.0000 0.0000 rg"));
}
#[test]
fn formula_short_circuit_and_precision_follow_the_designer() {
    let mut f = filled();
    f.report["report"]["formulas"] =
        json!([{"name":"Safe","expression":"IF(true, 0.1 + 0.2, 1 / 0)"}]);
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] = json!("[Safe]");
    assert!(has_text(&f.generate().unwrap(), "(0.3)"));
}
#[test]
fn invalid_background_division_and_unknown_functions_are_rejected() {
    let mut f = filled();
    f.report["report"]["header"]["backgroundExpression"] = json!("IF(true, 'red', '#ffffff')");
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
    f.report["report"]["header"]["backgroundExpression"] =
        json!("IF(1 / 0 > 1, '#ff0000', '#ffffff')");
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
    f.report["report"]["header"]["backgroundExpression"] = json!("UNKNOWN(1)");
    assert!(matches!(f.generate(), Err(Error::Unsupported(_))));
}
