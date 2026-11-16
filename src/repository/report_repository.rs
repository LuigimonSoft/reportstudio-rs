use super::{ReportDefinition, Result};
use std::{fs, path::Path};
/// Persistence boundary for reading report definitions and saving generated PDFs.
pub trait DocumentRepository {
    fn load_report(&self, path: &Path) -> Result<ReportDefinition>;
    fn save_pdf(&self, path: &Path, bytes: &[u8]) -> Result<()>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct FileDocumentRepository;
impl DocumentRepository for FileDocumentRepository {
    fn load_report(&self, path: &Path) -> Result<ReportDefinition> {
        Ok(ReportDefinition::from_json_value(serde_json::from_slice(
            &fs::read(path)?,
        )?))
    }
    fn save_pdf(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        fs::write(path, bytes)?;
        Ok(())
    }
}
