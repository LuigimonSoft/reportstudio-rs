//! Native PDF generation from Report Studio report definitions.
//! Architecture: application → services → repository → models.
pub mod application;
pub mod models;
pub mod repository;
pub mod services;
pub use application::{
    DataColumn, DataRow, DataTable, DataType, DataValue, Error, GenerateReportDefinitionInput,
    ReportData, ReportDefinition, ReportGenerator, ReportProject, Result,
};
