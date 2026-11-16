//! Equality joins for report data. NULL keys never match, and inputs are not mutated.
use crate::repository::{DataType, DataValue, Error, ReportData, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
struct Endpoint {
    table: String,
    field: String,
}
#[derive(Clone, Debug)]
struct Relation {
    from: Endpoint,
    to: Endpoint,
}
#[derive(Clone, Default)]
pub(super) struct RowContext {
    pub(super) rows: HashMap<String, usize>,
}
pub(super) struct Relations<'a> {
    data: &'a ReportData,
    edges: Vec<Relation>,
    root: Option<String>,
}
impl<'a> Relations<'a> {
    pub(super) fn new(project: &Value, data: &'a ReportData) -> Result<Self> {
        let mut edges = Vec::new();
        if let Some(relations) = project.get("relations") {
            let relations = relations
                .as_array()
                .ok_or_else(|| Error::InvalidReport("relations must be an array".into()))?;
            let mut pairs = HashSet::new();
            for relation in relations {
                let endpoint = |name: &str| -> Result<Endpoint> {
                    let source = &relation[name];
                    let table = source["dataset"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| {
                            Error::InvalidReport(format!("relation {name} dataset is missing"))
                        })?
                        .to_owned();
                    let field = source["field"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| {
                            Error::InvalidReport(format!("relation {name} field is missing"))
                        })?
                        .to_owned();
                    let dataset = data.table(&table).ok_or_else(|| {
                        Error::InvalidData(format!("missing related table: {table}"))
                    })?;
                    if !dataset.columns().iter().any(|column| column.name == field) {
                        return Err(Error::InvalidData(format!(
                            "unknown relationship field {table}.{field}"
                        )));
                    }
                    Ok(Endpoint { table, field })
                };
                let from = endpoint("from")?;
                let to = endpoint("to")?;
                if from.table == to.table {
                    return Err(Error::InvalidReport(
                        "relationship must connect different tables".into(),
                    ));
                }
                if !matches!(
                    relation["cardinality"].as_str(),
                    Some("1:1" | "1:n" | "n:1" | "n:n")
                ) {
                    return Err(Error::InvalidReport(
                        "invalid relationship cardinality".into(),
                    ));
                }
                let a = data
                    .table(&from.table)
                    .unwrap()
                    .columns()
                    .iter()
                    .find(|c| c.name == from.field)
                    .unwrap()
                    .data_type;
                let b = data
                    .table(&to.table)
                    .unwrap()
                    .columns()
                    .iter()
                    .find(|c| c.name == to.field)
                    .unwrap()
                    .data_type;
                if !compatible(a, b) {
                    return Err(Error::InvalidData(format!(
                        "incompatible relationship types: {}.{} and {}.{}",
                        from.table, from.field, to.table, to.field
                    )));
                }
                let mut key = [
                    (from.table.clone(), from.field.clone()),
                    (to.table.clone(), to.field.clone()),
                ];
                key.sort();
                if !pairs.insert(key) {
                    return Err(Error::InvalidReport("duplicate relationship".into()));
                }
                edges.push(Relation { from, to });
            }
        }
        let root = project["dataModel"]["rootDataset"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        if let Some(id) = &root {
            if data.table(id).is_none() {
                return Err(Error::InvalidData(format!("missing root table: {id}")));
            }
        }
        Ok(Self { data, edges, root })
    }
    /// Each context is one connected join combination starting at a specific row.
    pub(super) fn contexts(&self, table: &str, index: usize) -> Result<Vec<RowContext>> {
        let source = self
            .data
            .table(table)
            .ok_or_else(|| Error::InvalidData(format!("missing table: {table}")))?;
        if index >= source.rows().len() {
            return Err(Error::InvalidData(format!("invalid row for {table}")));
        }
        let mut states = vec![RowContext {
            rows: HashMap::from([(table.to_owned(), index)]),
        }];
        let mut bound = HashSet::from([table.to_owned()]);
        let mut used = HashSet::new();
        while let Some((edge_index, edge, forward)) = self
            .edges
            .iter()
            .enumerate()
            .filter(|(i, _)| !used.contains(i))
            .find_map(|(i, e)| {
                let a = bound.contains(&e.from.table);
                let b = bound.contains(&e.to.table);
                if a && !b {
                    Some((i, e, true))
                } else if b && !a {
                    Some((i, e, false))
                } else {
                    None
                }
            })
        {
            let (local, remote) = if forward {
                (&edge.from, &edge.to)
            } else {
                (&edge.to, &edge.from)
            };
            let local_table = self.data.table(&local.table).unwrap();
            let remote_table = self.data.table(&remote.table).unwrap();
            let mut next = Vec::new();
            for state in states {
                let local_row = &local_table.rows()[state.rows[&local.table]];
                let value = local_table.field(local_row, &local.field).unwrap();
                for (idx, other) in remote_table.rows().iter().enumerate() {
                    if equal(value, remote_table.field(other, &remote.field).unwrap()) {
                        let mut joined = state.clone();
                        joined.rows.insert(remote.table.clone(), idx);
                        next.push(joined);
                        if next.len() > 100_000 {
                            return Err(Error::InvalidData(
                                "relationship produces too many combinations".into(),
                            ));
                        }
                    }
                }
            }
            states = next;
            bound.insert(remote.table.clone());
            used.insert(edge_index);
        }
        states.retain(|state| {
            self.edges.iter().all(|e| {
                match (state.rows.get(&e.from.table), state.rows.get(&e.to.table)) {
                    (Some(a), Some(b)) => {
                        let ta = self.data.table(&e.from.table).unwrap();
                        let tb = self.data.table(&e.to.table).unwrap();
                        equal(
                            ta.field(&ta.rows()[*a], &e.from.field).unwrap(),
                            tb.field(&tb.rows()[*b], &e.to.field).unwrap(),
                        )
                    }
                    _ => true,
                }
            })
        });
        if let Some(root) = &self.root {
            // An unrelated root must not filter an independent detail source.
            if bound.contains(root) {
                states.retain(|state| state.rows.contains_key(root));
            }
        }
        Ok(states)
    }
    pub(super) fn first_context(&self) -> Result<RowContext> {
        let source = self
            .root
            .as_deref()
            .and_then(|id| self.data.table(id))
            .or_else(|| self.data.tables().iter().find(|t| !t.rows().is_empty()));
        if let Some(table) = source {
            if !table.rows().is_empty() {
                return Ok(self
                    .contexts(&table.name, 0)?
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| RowContext {
                        rows: HashMap::from([(table.name.clone(), 0)]),
                    }));
            }
        }
        Ok(RowContext::default())
    }
    fn linked_to_context(&self, context: &RowContext, target: &str) -> bool {
        let mut visited: HashSet<String> = context.rows.keys().cloned().collect();
        loop {
            if visited.contains(target) {
                return true;
            }
            let before = visited.len();
            for edge in &self.edges {
                if visited.contains(&edge.from.table) {
                    visited.insert(edge.to.table.clone());
                }
                if visited.contains(&edge.to.table) {
                    visited.insert(edge.from.table.clone());
                }
            }
            if visited.len() == before {
                return false;
            }
        }
    }
    pub(super) fn value<'b>(
        &'b self,
        context: &RowContext,
        table: &str,
        field: &str,
    ) -> Result<Option<&'b DataValue>> {
        let source = self
            .data
            .table(table)
            .ok_or_else(|| Error::InvalidData(format!("missing table: {table}")))?;
        if !source.columns().iter().any(|c| c.name == field) {
            return Err(Error::InvalidData(format!("unknown field {table}.{field}")));
        }
        let index = match context.rows.get(table) {
            Some(index) => *index,
            None if self.linked_to_context(context, table) => return Ok(None),
            None => 0,
        };
        Ok(source
            .rows()
            .get(index)
            .and_then(|row| source.field(row, field)))
    }
}
fn compatible(a: DataType, b: DataType) -> bool {
    a == b
        || matches!(
            (a, b),
            (DataType::Integer, DataType::Decimal)
                | (DataType::Decimal, DataType::Integer)
                | (DataType::Date, DataType::Text)
                | (DataType::Text, DataType::Date)
        )
}
fn equal(a: &DataValue, b: &DataValue) -> bool {
    match (a, b) {
        (DataValue::Null, _) | (_, DataValue::Null) => false,
        (DataValue::Integer(x), DataValue::Decimal(y))
        | (DataValue::Decimal(y), DataValue::Integer(x)) => {
            y.is_finite()
                && *y < 9_223_372_036_854_775_808.0
                && *y == (*x as f64)
                && (*y as i64) == *x
        }
        (DataValue::Text(x), DataValue::Date(y)) | (DataValue::Date(x), DataValue::Text(y)) => {
            x == y
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::equal;
    use crate::repository::DataValue;
    #[test]
    fn decimal_outside_integer_range_does_not_match_saturated_maximum() {
        assert!(!equal(
            &DataValue::Integer(i64::MAX),
            &DataValue::Decimal(9_223_372_036_854_775_808.0)
        ));
    }
    #[test]
    fn nulls_never_match_and_numeric_types_match_without_string_coercion() {
        assert!(!equal(&DataValue::Null, &DataValue::Null));
        assert!(equal(&DataValue::Integer(42), &DataValue::Decimal(42.0)));
        assert!(!equal(
            &DataValue::Integer(42),
            &DataValue::Text("42".into())
        ));
    }
}
