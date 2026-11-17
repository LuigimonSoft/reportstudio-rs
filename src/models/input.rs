use super::{ReportData, ReportDefinition};
use std::path::PathBuf;
/// Request to render a report file with caller-supplied data.
#[derive(Clone, Debug)]
pub struct GenerateReportInput {
    pub report_path: PathBuf,
    pub data: ReportData,
}
/// Request to render an in-memory report definition with caller-supplied data.
#[derive(Clone, Debug)]
pub struct GenerateReportDefinitionInput {
    pub report: ReportDefinition,
    pub data: ReportData,
}
