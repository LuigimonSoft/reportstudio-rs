//! Supplies deterministic data for grouped PDF browser verification.
use reportstudio::{DataTable, DataType, ReportData, ReportGenerator};
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(args.next().ok_or("expected output path")?);
    let report = PathBuf::from(args.next().ok_or("expected report path")?);
    let mut table = DataTable::new("details");
    table.add_column("description", DataType::Text)?;
    table.add_column("category", DataType::Text)?;
    table.add_column("amount", DataType::Integer)?;
    for n in 0..100 {
        table.add_row(vec![
            format!("Item{n:03}").into(),
            (if n % 2 == 0 { "North" } else { "South" }).into(),
            1.into(),
        ])?;
    }
    let mut data = ReportData::new();
    data.add_table(table)?;
    ReportGenerator::new(report)
        .with_data(data)
        .save_pdf(output)?;
    Ok(())
}
