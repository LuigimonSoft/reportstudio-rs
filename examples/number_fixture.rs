//! Deterministic numeric values for browser PDF comparisons.
use reportstudio::{DataTable, DataType, DataValue, ReportData, ReportGenerator};
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let destination = PathBuf::from(args.next().ok_or("expected output path")?);
    let report = PathBuf::from(args.next().ok_or("expected report path")?);
    let mut table = DataTable::new("details");
    table.add_column("amount", DataType::Decimal)?;
    table.add_column("ratio", DataType::Decimal)?;
    table.add_column("date", DataType::Date)?;
    table.add_row(vec![
        1500.25.into(),
        0.12567.into(),
        DataValue::Date("2026-01-02".into()),
    ])?;
    table.add_row(vec![
        (-2.675).into(),
        (-0.0075).into(),
        DataValue::Date("2025-12-31".into()),
    ])?;
    let mut data = ReportData::new();
    data.add_table(table)?;
    ReportGenerator::new(report)
        .with_data(data)
        .save_pdf(destination)?;
    Ok(())
}
