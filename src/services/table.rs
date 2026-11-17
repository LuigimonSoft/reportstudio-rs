//! Normalize the editable cell elements stored by the designer for one record.
use super::relations::{Relations, RowContext};
use crate::repository::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashMap;

struct TableLayout {
    top: f64,
    heights: Vec<f64>,
    definition: Value,
}
pub(super) struct PreparedElements {
    pub elements: Vec<Value>,
    pub has_tables: bool,
}
fn finite(value: &Value, name: &str) -> Result<f64> {
    value
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| Error::InvalidReport(format!("invalid table {name}")))
}
fn index(value: &Value, name: &str) -> Result<usize> {
    value
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| Error::InvalidReport(format!("invalid table {name}")))
}

pub(super) fn prepare(
    section: &Value,
    relations: &Relations<'_>,
    format: super::format::FormatContext<'_>,
    context: &RowContext,
    current: Option<&str>,
    compact: bool,
    first_row: bool,
) -> Result<PreparedElements> {
    let mut elements = section["elements"]
        .as_array()
        .ok_or_else(|| Error::InvalidReport("band has no elements array".into()))?
        .iter()
        .filter(|e| e["visible"] != false)
        .cloned()
        .collect::<Vec<_>>();
    let mut tables: HashMap<String, TableLayout> = HashMap::new();
    for element in &elements {
        if element.get("table").is_none() && element.get("cell").is_none() {
            continue;
        }
        let group = element["groupId"]
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::InvalidReport("table element is missing groupId".into()))?;
        let table = element
            .get("table")
            .filter(|v| v.is_object())
            .ok_or_else(|| Error::InvalidReport("table element is missing settings".into()))?;
        let row = index(&element["cell"]["row"], "cell row")?;
        let col = index(&element["cell"]["column"], "cell column")?;
        let row_count = index(&table["rows"], "row count")?;
        let col_count = index(&table["columns"], "column count")?;
        if row_count == 0
            || row_count > 20
            || col_count == 0
            || col_count > 12
            || row >= row_count
            || col >= col_count
        {
            return Err(Error::InvalidReport("invalid table cell dimensions".into()));
        }
        if let Some(styles) = table["styles"].as_object() {
            for style in styles.values() {
                if style
                    .get("backgroundExpression")
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.trim().is_empty())
                {
                    return Err(Error::Unsupported("table background expression".into()));
                }
            }
        }
        let y = finite(&element["y"], "element y")?;
        let layout = match tables.entry(group.to_owned()) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let default = finite(&table["rowHeight"], "row height")?;
                let heights = if let Some(rows) = table.get("rowHeights") {
                    let rows = rows
                        .as_array()
                        .ok_or_else(|| Error::InvalidReport("invalid table row heights".into()))?;
                    if rows.len() != row_count {
                        return Err(Error::InvalidReport("invalid table row heights".into()));
                    }
                    rows.iter()
                        .map(|h| finite(h, "row height"))
                        .collect::<Result<Vec<_>>>()?
                } else {
                    vec![default; row_count]
                };
                if heights.iter().any(|h| *h < 0.15 || *h > 1.0) {
                    return Err(Error::InvalidReport("invalid table row height".into()));
                }
                entry.insert(TableLayout {
                    top: y,
                    heights,
                    definition: table.clone(),
                })
            }
        };
        if layout.definition != *table {
            return Err(Error::InvalidReport(format!(
                "inconsistent settings in table {group}"
            )));
        }
        layout.top = layout.top.min(y);
        if element["type"] == "text" {
            let raw = element["text"]
                .as_str()
                .ok_or_else(|| Error::InvalidReport("text is missing".into()))?;
            let rendered = super::layout::resolve(
                raw,
                relations,
                context,
                current,
                &element["style"],
                format,
            )?;
            let width = finite(&element["width"], "element width")?;
            if width <= 0.0 {
                return Err(Error::InvalidReport("invalid table element width".into()));
            }
            let needed =
                super::text::required_height(&rendered, &element["style"], width * 72.0, 0.0)?
                    / 72.0
                    + 0.06;
            layout.heights[row] = layout.heights[row].max(needed);
        }
    }
    let has_tables = !tables.is_empty();
    let table_only = !elements.is_empty()
        && elements
            .iter()
            .all(|e| e.get("table").is_some() && e.get("cell").is_some());
    for element in &mut elements {
        let Some(group) = element["groupId"].as_str() else {
            continue;
        };
        let Some(table) = tables.get(group) else {
            continue;
        };
        let row = index(&element["cell"]["row"], "cell row")?;
        let is_text = element["type"] == "text";
        let y =
            table.top + table.heights[..row].iter().sum::<f64>() + if is_text { 0.03 } else { 0.0 };
        let height = table.heights[row] - if is_text { 0.06 } else { 0.0 };
        element["y"] = json!(y);
        element["height"] = json!(height);
    }
    if compact && table_only && !first_row {
        let leading = elements
            .iter()
            .map(|e| e["y"].as_f64().unwrap_or(0.0))
            .fold(f64::INFINITY, f64::min);
        if leading > 0.0 && leading.is_finite() {
            for element in &mut elements {
                element["y"] = json!(element["y"].as_f64().unwrap() - leading);
            }
        }
    }
    Ok(PreparedElements {
        elements,
        has_tables,
    })
}
