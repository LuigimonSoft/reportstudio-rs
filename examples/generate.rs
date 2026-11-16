use reportstudio::{DataTable, DataType, ReportData, ReportGenerator};
fn main() -> reportstudio::Result<()> {
    let mut details = DataTable::new("details");
    details.add_column("description", DataType::Text)?;
    details.add_row(vec!["Implementation consulting".into()])?;
    details.add_row(vec!["Report development".into()])?;
    let mut data = ReportData::new();
    data.add_table(details)?;
    ReportGenerator::new("examples/simple-report.json")
        .with_data(data)
        .save_pdf("example.pdf")?;
    println!("Created example.pdf");
    Ok(())
}
