use super::{Error, Result};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Text,
    Integer,
    Decimal,
    Boolean,
    Date,
}
#[derive(Clone, Debug, PartialEq)]
pub enum DataValue {
    Null,
    Text(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    Date(String),
}
impl From<&str> for DataValue {
    fn from(v: &str) -> Self {
        Self::Text(v.into())
    }
}
impl From<String> for DataValue {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}
impl From<i32> for DataValue {
    fn from(v: i32) -> Self {
        Self::Integer(v.into())
    }
}
impl From<i64> for DataValue {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}
impl From<f64> for DataValue {
    fn from(v: f64) -> Self {
        Self::Decimal(v)
    }
}
impl From<bool> for DataValue {
    fn from(v: bool) -> Self {
        Self::Boolean(v)
    }
}
impl DataValue {
    pub(crate) fn display(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Text(s) | Self::Date(s) => s.clone(),
            Self::Integer(v) => v.to_string(),
            Self::Decimal(v) => v.to_string(),
            Self::Boolean(v) => v.to_string(),
        }
    }
    fn matches(&self, ty: DataType) -> bool {
        match (self, ty) {
            (Self::Null, _)
            | (Self::Text(_), DataType::Text)
            | (Self::Integer(_), DataType::Integer | DataType::Decimal)
            | (Self::Boolean(_), DataType::Boolean) => true,
            (Self::Decimal(v), DataType::Decimal) => v.is_finite(),
            (Self::Date(v), DataType::Date) => valid_date(v),
            _ => false,
        }
    }
}
fn valid_date(v: &str) -> bool {
    let b = v.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let y = v[0..4].parse::<u32>().unwrap_or(0);
    let m = v[5..7].parse::<u32>().unwrap_or(0);
    let d = v[8..10].parse::<u32>().unwrap_or(0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    };
    y > 0 && d > 0 && d <= days
}
#[derive(Clone, Debug)]
pub struct DataColumn {
    pub name: String,
    pub data_type: DataType,
}
#[derive(Clone, Debug)]
pub struct DataRow {
    values: Vec<DataValue>,
}
impl DataRow {
    pub fn values(&self) -> &[DataValue] {
        &self.values
    }
}
#[derive(Clone, Debug)]
pub struct DataTable {
    pub name: String,
    columns: Vec<DataColumn>,
    rows: Vec<DataRow>,
}
impl DataTable {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: vec![],
            rows: vec![],
        }
    }
    pub fn add_column(
        &mut self,
        name: impl Into<String>,
        data_type: DataType,
    ) -> Result<&mut Self> {
        let name = name.into();
        if name.trim().is_empty() || self.columns.iter().any(|c| c.name == name) {
            return Err(Error::InvalidData(format!(
                "duplicate or empty column: {name}"
            )));
        }
        if !self.rows.is_empty() {
            return Err(Error::InvalidData(
                "columns must be defined before rows".into(),
            ));
        }
        self.columns.push(DataColumn { name, data_type });
        Ok(self)
    }
    pub fn add_row(&mut self, values: Vec<DataValue>) -> Result<&mut Self> {
        if self.columns.is_empty() {
            return Err(Error::InvalidData(
                "table requires at least one column".into(),
            ));
        }
        if values.len() != self.columns.len() {
            return Err(Error::InvalidData(format!(
                "table {}: expected {} values, got {}",
                self.name,
                self.columns.len(),
                values.len()
            )));
        }
        for (column, value) in self.columns.iter().zip(&values) {
            if !value.matches(column.data_type) {
                return Err(Error::InvalidData(format!(
                    "table {}, column {}: wrong type or invalid value",
                    self.name, column.name
                )));
            }
        }
        self.rows.push(DataRow { values });
        Ok(self)
    }
    pub fn columns(&self) -> &[DataColumn] {
        &self.columns
    }
    pub fn rows(&self) -> &[DataRow] {
        &self.rows
    }
    pub(crate) fn field<'a>(&self, row: &'a DataRow, name: &str) -> Option<&'a DataValue> {
        self.columns
            .iter()
            .position(|c| c.name == name)
            .and_then(|i| row.values.get(i))
    }
}
#[derive(Clone, Debug, Default)]
pub struct ReportData {
    tables: Vec<DataTable>,
}
impl ReportData {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_table(&mut self, table: DataTable) -> Result<&mut Self> {
        if table.name.trim().is_empty() || self.tables.iter().any(|t| t.name == table.name) {
            return Err(Error::InvalidData(format!(
                "duplicate or empty table: {}",
                table.name
            )));
        }
        let names: HashSet<_> = table.columns.iter().map(|c| &c.name).collect();
        if names.len() != table.columns.len() {
            return Err(Error::InvalidData("duplicate column".into()));
        }
        self.tables.push(table);
        Ok(self)
    }
    pub fn tables(&self) -> &[DataTable] {
        &self.tables
    }
    pub(crate) fn table(&self, name: &str) -> Option<&DataTable> {
        self.tables.iter().find(|t| t.name == name)
    }
}
