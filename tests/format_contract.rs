mod support;
use reportstudio::{DataTable, DataType, DataValue, Error};
use serde_json::json;
use support::*;
fn fixture(value: DataValue, ty: DataType, style: &str, locale: &str, currency: &str) -> Fixture {
    let mut f = Fixture::new();
    let mut table = DataTable::new("details");
    table.add_column("amount", ty).unwrap();
    table.add_row(vec![value]).unwrap();
    f.data.add_table(table).unwrap();
    f.report["datasets"][0]["schema"]["fields"] = json!([{"name":"amount","type":if matches!(ty,DataType::Decimal|DataType::Integer){"number"}else{"text"}}]);
    f.report["report"]["locale"] = json!(locale);
    f.report["report"]["currency"] = json!(currency);
    let element = &mut f.report["report"]["details"][0]["band"]["elements"][0];
    element["text"] = json!("Value: [amount]");
    element["style"]["format"] = json!(style);
    f
}
#[test]
fn currency_rounds_and_preserves_literal_text() {
    let f = fixture(1.005.into(), DataType::Decimal, "currency", "en-US", "USD");
    assert!(has_text(&f.generate().unwrap(), "Value: $1.01"));
}
#[test]
fn integer_keeps_full_precision() {
    let f = fixture(i64::MAX.into(), DataType::Integer, "number", "en-US", "USD");
    assert!(has_text(
        &f.generate().unwrap(),
        "9,223,372,036,854,775,807"
    ));
}
#[test]
fn mexican_currency_uses_expected_symbols() {
    let f = fixture(1234.into(), DataType::Integer, "currency", "es-MX", "MXN");
    assert!(has_text(&f.generate().unwrap(), "$1,234.00"));
    let f = fixture(1234.into(), DataType::Integer, "currency", "es-MX", "USD");
    assert!(has_text(&f.generate().unwrap(), r"USD\2401,234.00"));
}
#[test]
fn percentage_is_scaled_and_rounded() {
    let f = fixture(0.12567.into(), DataType::Decimal, "percent", "en-US", "USD");
    assert!(has_text(&f.generate().unwrap(), "12.57%"));
}
#[test]
fn yen_has_no_fraction_digits() {
    let f = fixture(1500.5.into(), DataType::Decimal, "currency", "en-US", "JPY");
    assert!(has_text(&f.generate().unwrap(), r"\2451,501"));
}
#[test]
fn number_format_does_not_transform_text() {
    let f = fixture("001234".into(), DataType::Text, "number", "en-US", "USD");
    assert!(has_text(&f.generate().unwrap(), "001234"));
}
#[test]
fn date_formats_valid_values_and_leaves_invalid_dates_literal() {
    let f = fixture(
        DataValue::Date("2024-02-29".into()),
        DataType::Date,
        "date",
        "en-US",
        "USD",
    );
    assert!(has_text(&f.generate().unwrap(), "2/29/2024"));
    let f = fixture("2025-02-29".into(), DataType::Text, "date", "en-US", "USD");
    assert!(has_text(&f.generate().unwrap(), "2025-02-29"));
}
#[test]
fn unsupported_locales_and_currencies_fail_explicitly() {
    let f = fixture(1.into(), DataType::Integer, "number", "fr-FR", "EUR");
    assert!(matches!(f.generate(), Err(Error::Unsupported(_))));
    let f = fixture(1.into(), DataType::Integer, "currency", "en-US", "XXX");
    assert!(matches!(f.generate(), Err(Error::Unsupported(_))));
}
#[test]
fn null_stays_empty() {
    let f = fixture(
        DataValue::Null,
        DataType::Decimal,
        "currency",
        "en-US",
        "USD",
    );
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Value: "));
    assert!(!has_text(&pdf, "$0"));
}
