//! Native PDF generation from Report Studio report definitions.
//! Architecture: application → services → repository → models.
pub mod application;
pub mod models;
pub mod repository;
pub mod services;
pub use application::{
    DataColumn, DataRow, DataTable, DataType, DataValue, Error, ReportData, ReportGenerator, Result,
};
