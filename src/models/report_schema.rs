//! Editable, serializable model for the core Report Studio project schema.
//!
//! Advanced settings and unrecognized properties remain JSON values so that
//! newer report definitions are not silently discarded. Rendering still
//! validates the same feature set as a report loaded from a file.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

type Extra = Map<String, Value>;

/// A complete report project, including its report, datasets and relationships.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportProject {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    pub report: ReportModel,
    #[serde(default)]
    pub datasets: Vec<DatasetModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relations: Option<Vec<RelationModel>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<ParameterModel>>,
    /// Designer-specific data-model settings, preserved as JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_model: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub page: PageModel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<FormulaModel>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<BandModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_header: Option<BandModel>,
    #[serde(default)]
    pub details: Vec<DetailModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_footer: Option<BandModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<BandModel>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageModel {
    pub width: f64,
    pub height: f64,
    pub margins: MarginsModel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MarginsModel {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetModel {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub schema: DatasetSchemaModel,
    /// Sample rows are retained in the model; the renderer uses supplied ReportData.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<Value>>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetSchemaModel {
    #[serde(default)]
    pub fields: Vec<FieldModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested_source: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldModel {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sql_type: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub height: f64,
    #[serde(default)]
    pub elements: Vec<ElementModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<BandModel>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_break_before: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility_expression: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_expression: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub element_type: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// Text style and advanced element settings retain their full JSON shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub dataset: String,
    pub band: BandModel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_direction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_header: Option<BandModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_footer: Option<BandModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<Value>>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FormulaModel {
    pub name: String,
    pub expression: String,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RelationModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub from: EndpointModel,
    pub to: EndpointModel,
    pub cardinality: String,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EndpointModel {
    pub dataset: String,
    pub field: String,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterModel {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub parameter_type: Option<String>,
    pub dataset: String,
    pub field: String,
    pub operator: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_value: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(flatten)]
    pub extra: Extra,
}
