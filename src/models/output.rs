/// Result of rendering a report, independent of where it is stored.
#[derive(Clone, Debug)]
pub struct GenerateReportOutput {
    pub pdf_bytes: Vec<u8>,
}
