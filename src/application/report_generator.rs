use crate::services::{
    GenerateReportDefinitionInput, GenerateReportInput, GenerateReportOutput, ReportData,
    ReportDefinition, ReportProject, ReportService, Result,
};
use std::path::{Path, PathBuf};

enum Source {
    File(GenerateReportInput),
    Definition(GenerateReportDefinitionInput),
}
/// Application entry point for rendering a report file or in-memory definition as a PDF.
pub struct ReportGenerator {
    source: Source,
}
impl ReportGenerator {
    /// Load the report from a file each time it is rendered.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            source: Source::File(GenerateReportInput {
                report_path: PathBuf::from(path.as_ref()),
                data: ReportData::new(),
            }),
        }
    }
    /// Use an in-memory report definition without reading a report file.
    pub fn from_definition(report: ReportDefinition) -> Self {
        Self {
            source: Source::Definition(GenerateReportDefinitionInput {
                report,
                data: ReportData::new(),
            }),
        }
    }
    /// Convert the typed project model into an in-memory report generator.
    pub fn from_model(model: &ReportProject) -> Result<Self> {
        Ok(Self::from_definition(ReportDefinition::from_serializable(
            model,
        )?))
    }
    pub fn with_data(mut self, data: ReportData) -> Self {
        match &mut self.source {
            Source::File(input) => input.data = data,
            Source::Definition(input) => input.data = data,
        }
        self
    }
    fn generate(&self, service: &ReportService) -> Result<GenerateReportOutput> {
        match &self.source {
            Source::File(input) => service.generate(input),
            Source::Definition(input) => service.generate_from_definition(input),
        }
    }
    pub fn to_pdf_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.generate(&ReportService::default())?.pdf_bytes)
    }
    pub fn save_pdf(&self, path: impl AsRef<Path>) -> Result<()> {
        let service = ReportService::default();
        let output = self.generate(&service)?;
        service.save(&output, path.as_ref())
    }
}
