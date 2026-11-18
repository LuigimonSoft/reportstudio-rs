//! Generate an AWB PDF from a report template and a separate JSON data file.
#[path = "awb/data_loader.rs"]
mod data_loader;

use reportstudio::ReportGenerator;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let destination = PathBuf::from(args.next().ok_or("expected output PDF path")?);
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/awb");
    let report_path = example.join("awb-template.json");
    let data_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| example.join("awb-data.json"));
    if args.next().is_some() {
        return Err("usage: awb_fixture <output.pdf> [data.json]".into());
    }

    let data = data_loader::load_report_data(&report_path, &data_path)?;
    ReportGenerator::new(&report_path)
        .with_data(data)
        .save_pdf(&destination)?;
    println!(
        "Generated {} using {}",
        destination.display(),
        data_path.display()
    );
    Ok(())
}
