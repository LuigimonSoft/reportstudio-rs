use serde_json::Value;

/// Parsed report definition. Validation is performed by the report service.
#[derive(Clone, Debug)]
pub struct ReportDefinition {
    document: Value,
}
impl ReportDefinition {
    pub fn from_json_value(document: Value) -> Self {
        Self { document }
    }
    pub fn as_json_value(&self) -> &Value {
        &self.document
    }
}
