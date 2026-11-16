//! Deterministic report data for the browser PDF verification suite.
use reportstudio::{DataTable, DataType, ReportData, ReportGenerator};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let destination = PathBuf::from(args.next().ok_or("expected output path")?);
    let first = args
        .next()
        .and_then(|s| s.into_string().ok())
        .unwrap_or_else(|| "Implementation consulting".into());
    let mut table = DataTable::new("details");
    table.add_column("description", DataType::Text)?;
    table.add_row(vec![first.into()])?;
    table.add_row(vec!["Report development".into()])?;
    let mut data = ReportData::new();
    data.add_table(table)?;
    let report = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-report.json")
    });
    ReportGenerator::new(report)
        .with_data(data)
        .save_pdf(destination)?;
    Ok(())
}
