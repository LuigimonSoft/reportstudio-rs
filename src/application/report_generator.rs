use crate::services::{GenerateReportInput, ReportData, ReportService, Result};
use std::path::{Path, PathBuf};
/// Application entry point for rendering a report JSON file as a PDF.
pub struct ReportGenerator {
    input: GenerateReportInput,
}
impl ReportGenerator {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            input: GenerateReportInput {
                report_path: PathBuf::from(path.as_ref()),
                data: ReportData::new(),
            },
        }
    }
    pub fn with_data(mut self, data: ReportData) -> Self {
        self.input.data = data;
        self
    }
    pub fn to_pdf_bytes(&self) -> Result<Vec<u8>> {
        Ok(ReportService::default().generate(&self.input)?.pdf_bytes)
    }
    pub fn save_pdf(&self, path: impl AsRef<Path>) -> Result<()> {
        let service = ReportService::default();
        let output = service.generate(&self.input)?;
        service.save(&output, path.as_ref())
    }
}
