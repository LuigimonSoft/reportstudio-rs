use super::{GenerateReportInput, GenerateReportOutput, Result};
use crate::repository::{DocumentRepository, FileDocumentRepository};
use std::path::Path;
/// Coordinates report loading, layout and PDF persistence.
pub struct ReportService<R = FileDocumentRepository> {
    repository: R,
}
impl Default for ReportService<FileDocumentRepository> {
    fn default() -> Self {
        Self {
            repository: FileDocumentRepository,
        }
    }
}
impl<R: DocumentRepository> ReportService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
    pub fn generate(&self, input: &GenerateReportInput) -> Result<GenerateReportOutput> {
        let report = self.repository.load_report(&input.report_path)?;
        Ok(GenerateReportOutput {
            pdf_bytes: super::layout::render(report.as_json_value(), &input.data)?,
        })
    }
    pub fn save(&self, output: &GenerateReportOutput, path: &Path) -> Result<()> {
        self.repository.save_pdf(path, &output.pdf_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::ReportService;
    use crate::repository::{
        DataTable, DataType, DocumentRepository, Error, GenerateReportInput, ReportData,
        ReportDefinition, Result,
    };
    use serde_json::{json, Value};
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
        rc::Rc,
    };

    type SavedWrites = Rc<RefCell<Vec<Vec<u8>>>>;

    struct FakeRepository {
        document: Value,
        writes: SavedWrites,
    }
    impl DocumentRepository for FakeRepository {
        fn load_report(&self, _: &Path) -> Result<ReportDefinition> {
            Ok(ReportDefinition::from_json_value(self.document.clone()))
        }
        fn save_pdf(&self, _: &Path, bytes: &[u8]) -> Result<()> {
            self.writes.borrow_mut().push(bytes.to_vec());
            Ok(())
        }
    }
    fn fixture() -> (
        ReportService<FakeRepository>,
        GenerateReportInput,
        SavedWrites,
    ) {
        let writes = Rc::new(RefCell::new(Vec::new()));
        let document = json!({"version":1,"report":{"page":{"width":8.5,"height":11,"margins":{"top":0.5,"right":0.5,"bottom":0.5,"left":0.5}},"details":[{"dataset":"details","band":{"height":0.3,"elements":[{"type":"text","x":0,"y":0,"width":2,"height":0.2,"text":"[name]","style":{"fontsize":9}}]}}]}});
        let mut table = DataTable::new("details");
        table.add_column("name", DataType::Text).unwrap();
        table.add_row(vec!["Item".into()]).unwrap();
        let mut data = ReportData::new();
        data.add_table(table).unwrap();
        (
            ReportService::new(FakeRepository {
                document,
                writes: writes.clone(),
            }),
            GenerateReportInput {
                report_path: PathBuf::from("virtual.json"),
                data,
            },
            writes,
        )
    }
    #[test]
    fn service_renders_from_repository_and_saves_the_output() {
        let (service, input, writes) = fixture();
        let output = service.generate(&input).unwrap();
        assert!(output.pdf_bytes.starts_with(b"%PDF-"));
        assert!(writes.borrow().is_empty());
        service.save(&output, Path::new("virtual.pdf")).unwrap();
        assert_eq!(writes.borrow()[0], output.pdf_bytes);
    }
    #[test]
    fn failed_render_never_writes_an_output() {
        let (service, mut input, writes) = fixture();
        input.data = ReportData::new();
        assert!(matches!(
            service.generate(&input),
            Err(Error::InvalidData(_))
        ));
        assert!(writes.borrow().is_empty());
    }
}
