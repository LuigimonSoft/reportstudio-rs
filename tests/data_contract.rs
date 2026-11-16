use reportstudio::{DataTable, DataType, DataValue, ReportData};
#[test]
fn columns_preserve_names_types_and_order() {
    let mut t = DataTable::new("goods");
    t.add_column("name", DataType::Text).unwrap();
    t.add_column("qty", DataType::Integer).unwrap();
    assert_eq!(t.name, "goods");
    assert_eq!(t.columns()[0].name, "name");
    assert_eq!(t.columns()[1].data_type, DataType::Integer);
}
#[test]
fn rows_preserve_insertion_order_and_values() {
    let mut t = DataTable::new("goods");
    t.add_column("name", DataType::Text).unwrap();
    t.add_row(vec!["one".into()]).unwrap();
    t.add_row(vec!["two".into()]).unwrap();
    assert_eq!(t.rows()[0].values(), &[DataValue::Text("one".into())]);
    assert_eq!(t.rows()[1].values(), &[DataValue::Text("two".into())]);
}
#[test]
fn all_types_accept_null() {
    for ty in [
        DataType::Text,
        DataType::Integer,
        DataType::Decimal,
        DataType::Boolean,
        DataType::Date,
    ] {
        let mut t = DataTable::new("t");
        t.add_column("c", ty).unwrap();
        t.add_row(vec![DataValue::Null]).unwrap();
    }
}
#[test]
fn integer_values_are_accepted_by_decimal_columns() {
    let mut t = DataTable::new("t");
    t.add_column("c", DataType::Decimal).unwrap();
    t.add_row(vec![12.into()]).unwrap();
}
#[test]
fn decimal_is_not_implicitly_narrowed_to_integer() {
    let mut t = DataTable::new("t");
    t.add_column("c", DataType::Integer).unwrap();
    assert!(t.add_row(vec![1.5.into()]).is_err());
}
#[test]
fn nonfinite_decimals_are_rejected_without_appending() {
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut t = DataTable::new("t");
        t.add_column("c", DataType::Decimal).unwrap();
        assert!(t.add_row(vec![n.into()]).is_err());
        assert!(t.rows().is_empty());
    }
}
#[test]
fn invalid_row_does_not_change_existing_rows() {
    let mut t = DataTable::new("t");
    t.add_column("c", DataType::Integer).unwrap();
    t.add_row(vec![1.into()]).unwrap();
    assert!(t.add_row(vec!["wrong".into()]).is_err());
    assert_eq!(t.rows().len(), 1);
}
#[test]
fn wrong_number_of_values_is_rejected() {
    let mut t = DataTable::new("t");
    t.add_column("c", DataType::Text).unwrap();
    assert!(t.add_row(vec![]).is_err());
    assert!(t.add_row(vec!["a".into(), "b".into()]).is_err());
    assert!(t.rows().is_empty());
}
#[test]
fn empty_and_duplicate_columns_are_rejected() {
    let mut t = DataTable::new("t");
    assert!(t.add_column("  ", DataType::Text).is_err());
    t.add_column("c", DataType::Text).unwrap();
    assert!(t.add_column("c", DataType::Text).is_err());
    assert_eq!(t.columns().len(), 1);
}
#[test]
fn cannot_add_columns_after_first_row() {
    let mut t = DataTable::new("t");
    t.add_column("c", DataType::Text).unwrap();
    t.add_row(vec!["x".into()]).unwrap();
    assert!(t.add_column("d", DataType::Text).is_err());
    assert_eq!(t.columns().len(), 1);
}
#[test]
fn report_data_rejects_blank_and_duplicate_table_names() {
    let mut d = ReportData::new();
    assert!(d.add_table(DataTable::new(" ")).is_err());
    d.add_table(DataTable::new("one")).unwrap();
    assert!(d.add_table(DataTable::new("one")).is_err());
    assert_eq!(d.tables().len(), 1);
}
#[test]
fn valid_dates_include_leap_year_and_calendar_boundaries() {
    for date in ["2000-02-29", "2024-02-29", "2025-12-31", "0001-01-01"] {
        let mut t = DataTable::new("t");
        t.add_column("date", DataType::Date).unwrap();
        assert!(
            t.add_row(vec![DataValue::Date(date.into())]).is_ok(),
            "{date}"
        );
    }
}
#[test]
fn invalid_dates_and_non_ascii_dates_are_rejected() {
    for date in [
        "1900-02-29",
        "2025-02-29",
        "2024-04-31",
        "2024-00-01",
        "2024-13-01",
        "0000-01-01",
        "2024-1-01",
        "é024-01-01",
        "2024-01-00",
    ] {
        let mut t = DataTable::new("t");
        t.add_column("date", DataType::Date).unwrap();
        assert!(
            t.add_row(vec![DataValue::Date(date.into())]).is_err(),
            "{date}"
        );
    }
}
#[test]
fn text_and_date_are_not_interchangeable() {
    let mut t = DataTable::new("t");
    t.add_column("date", DataType::Date).unwrap();
    assert!(t.add_row(vec!["2024-01-01".into()]).is_err());
}
#[test]
fn multiple_tables_preserve_registration_order() {
    let mut d = ReportData::new();
    d.add_table(DataTable::new("parent")).unwrap();
    d.add_table(DataTable::new("child")).unwrap();
    assert_eq!(
        d.tables()
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        vec!["parent", "child"]
    );
}
