mod support;
use reportstudio::{DataTable, DataType};
use serde_json::json;
use support::*;
fn grouped() -> Fixture {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Text).unwrap();
    t.add_column("category", DataType::Text).unwrap();
    t.add_column("amount", DataType::Integer).unwrap();
    for (name, group, n) in [
        ("B1", "South", 2),
        ("A1", "North", 10),
        ("B2", "South", 3),
        ("A2", "North", 20),
    ] {
        t.add_row(vec![name.into(), group.into(), n.into()])
            .unwrap();
    }
    f.data.add_table(t).unwrap();
    f.report["report"]["details"][0]["groupBy"] = json!("category");
    f.report["report"]["details"][0]["groupHeader"] =
        json!({"id":"gh","height":0.3,"elements":[text("Group [category]")]});
    f.report["report"]["details"][0]["groupFooter"] =
        json!({"id":"gf","height":0.3,"elements":[text("Subtotal [SUM(amount)]")]});
    f
}
#[test]
fn groups_sort_rows_and_aggregate_within_each_group() {
    let pdf = grouped().generate().unwrap();
    let s = String::from_utf8_lossy(&pdf);
    let north = s.find("Group North").unwrap();
    let a1 = s.find("A1").unwrap();
    let a2 = s.find("A2").unwrap();
    let total1 = s.find("Subtotal 30").unwrap();
    let south = s.find("Group South").unwrap();
    let b1 = s.find("B1").unwrap();
    let b2 = s.find("B2").unwrap();
    let total2 = s.find("Subtotal 5").unwrap();
    assert!(
        north < a1
            && a1 < a2
            && a2 < total1
            && total1 < south
            && south < b1
            && b1 < b2
            && b2 < total2
    );
}
#[test]
fn groups_follow_descending_order() {
    let mut f = grouped();
    f.report["report"]["details"][0]["sortDirection"] = json!("desc");
    let pdf = f.generate().unwrap();
    let s = String::from_utf8_lossy(&pdf);
    assert!(s.find("Group South") < s.find("Group North"));
}
#[test]
fn group_header_repeats_and_totals_span_pages() {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Text).unwrap();
    t.add_column("category", DataType::Text).unwrap();
    t.add_column("amount", DataType::Integer).unwrap();
    for n in 0..100 {
        t.add_row(vec![format!("Item{n:03}").into(), "North".into(), 1.into()])
            .unwrap();
    }
    f.data.add_table(t).unwrap();
    f.report["report"]["details"][0]["groupBy"] = json!("category");
    f.report["report"]["details"][0]["groupHeader"] =
        json!({"id":"gh","height":0.3,"elements":[text("Group [category]")]});
    f.report["report"]["details"][0]["groupFooter"] =
        json!({"id":"gf","height":0.3,"elements":[text("Subtotal [SUM(amount)]")]});
    let pdf = f.generate().unwrap();
    let s = String::from_utf8_lossy(&pdf);
    let pages = s.matches("/Type /Page ").count();
    assert!(pages >= 3);
    assert_eq!(s.matches("(Group North)").count(), pages);
    assert_eq!(s.matches("(Subtotal 100)").count(), 1);
    for n in 0..100 {
        assert_eq!(s.matches(&format!("(Item{n:03})")).count(), 1);
    }
}
