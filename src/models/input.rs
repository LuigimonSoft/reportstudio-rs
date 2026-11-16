use super::ReportData;
use std::path::PathBuf;
/// Request to render a report file with caller-supplied data.
#[derive(Clone, Debug)]
pub struct GenerateReportInput {
    pub report_path: PathBuf,
    pub data: ReportData,
}
