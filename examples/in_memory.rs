//! Generate a PDF without loading a report file at runtime.
use reportstudio::{DataTable, DataType, ReportData, ReportGenerator, ReportProject};
use std::path::PathBuf;
fn main() -> reportstudio::Result<()> {
    // The typed project can also be constructed and changed by application code.
    let report: ReportProject = serde_json::from_str(include_str!("simple-report.json"))?;
    let mut details = DataTable::new("details");
    details.add_column("description", DataType::Text)?;
    details.add_row(vec!["Implementation consulting".into()])?;
    details.add_row(vec!["Report development".into()])?;
    let mut data = ReportData::new();
    data.add_table(details)?;
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("in-memory.pdf"));
    ReportGenerator::from_model(&report)?
        .with_data(data)
        .save_pdf(output)
}
