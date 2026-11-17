use super::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An in-memory Report Studio JSON document.
///
/// The document retains every JSON property. Its report schema and supported
/// features are validated during generation, just as for file-based reports.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReportDefinition {
    document: Value,
}
impl ReportDefinition {
    pub fn from_json_value(document: Value) -> Self {
        Self { document }
    }
    /// Parse JSON text without reading a file. Report validation occurs when rendered.
    pub fn from_json_str(json: &str) -> Result<Self> {
        Ok(Self::from_json_value(serde_json::from_str(json)?))
    }
    /// Convert an application-defined serializable report model into a definition.
    pub fn from_serializable<T: Serialize>(model: &T) -> Result<Self> {
        Ok(Self::from_json_value(serde_json::to_value(model)?))
    }
    pub fn as_json_value(&self) -> &Value {
        &self.document
    }
}
