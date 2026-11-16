mod support;
use reportstudio::{DataTable, DataType, DataValue, Error};
use serde_json::json;
use support::*;

fn project(parent_rows: Vec<Vec<DataValue>>, child_rows: Vec<Vec<DataValue>>) -> Fixture {
    let mut f = Fixture::new();
    let mut parent = DataTable::new("parents");
    parent.add_column("id", DataType::Integer).unwrap();
    parent.add_column("name", DataType::Text).unwrap();
    for row in parent_rows {
        parent.add_row(row).unwrap();
    }
    let mut child = DataTable::new("details");
    child.add_column("parentId", DataType::Decimal).unwrap();
    child.add_column("name", DataType::Text).unwrap();
    for row in child_rows {
        child.add_row(row).unwrap();
    }
    f.data.add_table(parent).unwrap();
    f.data.add_table(child).unwrap();
    f.report["relations"] = json!([{"id":"r","from":{"dataset":"parents","field":"id"},"to":{"dataset":"details","field":"parentId"},"cardinality":"1:n"}]);
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] =
        json!("[parents.name]: [name]");
    f
}
#[test]
fn related_fields_follow_each_detail_row_without_mutating_input() {
    let f = project(
        vec![
            vec![1.into(), "First parent".into()],
            vec![2.into(), "Second parent".into()],
        ],
        vec![
            vec![1.into(), "First child".into()],
            vec![2.into(), "Second child".into()],
        ],
    );
    let before = f.data.tables()[0].rows()[0].values().to_vec();
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "First parent: First child"));
    assert!(has_text(&pdf, "Second parent: Second child"));
    assert!(!has_text(&pdf, "First parent: Second child"));
    assert_eq!(f.data.tables()[0].rows()[0].values(), before);
}
#[test]
fn unmatched_or_null_keys_do_not_match_even_when_both_are_null() {
    let f = project(
        vec![
            vec![DataValue::Null, "Null parent".into()],
            vec![1.into(), "Real parent".into()],
        ],
        vec![
            vec![DataValue::Null, "Null child".into()],
            vec![9.into(), "Orphan".into()],
            vec![1.into(), "Linked".into()],
        ],
    );
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Real parent: Linked"));
    assert!(!has_text(&pdf, "Null child"));
    assert!(!has_text(&pdf, "Orphan"));
}
#[test]
fn no_coercion_occurs_between_boolean_and_numeric_keys() {
    let mut f = Fixture::new();
    let mut a = DataTable::new("a");
    a.add_column("id", DataType::Boolean).unwrap();
    a.add_row(vec![true.into()]).unwrap();
    let mut b = DataTable::new("details");
    b.add_column("parentId", DataType::Integer).unwrap();
    b.add_column("name", DataType::Text).unwrap();
    b.add_row(vec![1.into(), "wrong".into()]).unwrap();
    f.data.add_table(a).unwrap();
    f.data.add_table(b).unwrap();
    f.report["relations"] = json!([{"id":"r","from":{"dataset":"a","field":"id"},"to":{"dataset":"details","field":"parentId"},"cardinality":"1:n"}]);
    assert!(matches!(f.generate(), Err(Error::InvalidData(_))));
}
#[test]
fn one_to_many_join_keeps_each_match() {
    let f = project(
        vec![vec![1.into(), "Parent".into()]],
        vec![
            vec![1.into(), "Child A".into()],
            vec![1.into(), "Child B".into()],
        ],
    );
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Parent: Child A"));
    assert!(has_text(&pdf, "Parent: Child B"));
}
#[test]
fn chained_relations_resolve_qualified_fields_in_header_and_details() {
    let mut f = project(
        vec![vec![1.into(), "Parent".into()]],
        vec![vec![1.into(), "Child".into()]],
    );
    let mut countries = DataTable::new("countries");
    countries.add_column("name", DataType::Text).unwrap();
    countries.add_column("label", DataType::Text).unwrap();
    countries
        .add_row(vec!["Parent".into(), "Canada".into()])
        .unwrap();
    f.data.add_table(countries).unwrap();
    f.report["relations"].as_array_mut().unwrap().push(json!({"id":"r2","from":{"dataset":"parents","field":"name"},"to":{"dataset":"countries","field":"name"},"cardinality":"n:1"}));
    f.report["dataModel"] = json!({"rootDataset":"parents","tables":{}});
    f.report["report"]["header"]["elements"][0]["text"] = json!("[countries.label]");
    f.report["report"]["details"][0]["band"]["elements"][0]["text"] =
        json!("[countries.label]: [name]");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Canada: Child"));
    assert!(has_text(&pdf, "Canada"));
}
#[test]
fn invalid_relationship_field_is_an_explicit_error() {
    let mut f = project(
        vec![vec![1.into(), "Parent".into()]],
        vec![vec![1.into(), "Child".into()]],
    );
    f.report["relations"][0]["to"]["field"] = json!("absent");
    assert!(matches!(f.generate(), Err(Error::InvalidData(_))));
}
#[test]
fn missing_parent_table_is_an_explicit_error() {
    let mut f = Fixture::new();
    f.data.add_table(table("details", &["Child"])).unwrap();
    f.report["relations"] = json!([{"id":"r","from":{"dataset":"parents","field":"id"},"to":{"dataset":"details","field":"name"},"cardinality":"1:n"}]);
    assert!(matches!(f.generate(), Err(Error::InvalidData(_))));
}
#[test]
fn duplicate_relationship_is_rejected_in_both_directions() {
    let mut f = project(
        vec![vec![1.into(), "Parent".into()]],
        vec![vec![1.into(), "Child".into()]],
    );
    f.report["relations"].as_array_mut().unwrap().push(json!({"id":"reverse","from":{"dataset":"details","field":"parentId"},"to":{"dataset":"parents","field":"id"},"cardinality":"n:1"}));
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
#[test]
fn header_does_not_fall_back_to_an_unrelated_record_of_a_linked_table() {
    let mut f = project(
        vec![vec![1.into(), "Parent".into()]],
        vec![vec![2.into(), "Wrong child".into()]],
    );
    f.report["dataModel"] = json!({"rootDataset":"parents","tables":{}});
    f.report["report"]["header"]["elements"][0]["text"] = json!("Child: [details.name]");
    let pdf = f.generate().unwrap();
    assert!(!has_text(&pdf, "Wrong child"));
}
