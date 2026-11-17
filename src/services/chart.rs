//! Data-bound horizontal and vertical bar charts rendered as PDF vectors.
use super::relations::Relations;
use crate::repository::{DataType, DataValue, Error, Result};
use serde_json::{json, Value};

struct Point {
    label: String,
    values: Vec<f64>,
}
fn rgb(source: &str) -> Result<(f64, f64, f64)> {
    let (r, g, b) = super::layout::parse_color(source)?;
    Ok((
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
    ))
}
fn label(
    out: &mut String,
    text: &str,
    x: f64,
    top: f64,
    width: f64,
    height: f64,
    size: f64,
) -> Result<()> {
    super::text::render(
        super::text::TextBox {
            text,
            style: &json!({"fontsize":size,"color":"#334155","wrap":false}),
            x,
            top,
            width,
            height,
            color: (0.20, 0.25, 0.33),
        },
        out,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    chart: &Value,
    relations: &Relations<'_>,
    x: f64,
    top: f64,
    width: f64,
    height: f64,
    out: &mut String,
) -> Result<()> {
    let kind = chart["kind"].as_str().unwrap_or("");
    if !matches!(kind, "bar" | "column") {
        return Err(Error::Unsupported(format!("chart type {kind}")));
    }
    let source = chart["dataset"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::InvalidReport("chart dataset is missing".into()))?;
    let field = chart["category"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::InvalidReport("chart category is missing".into()))?;
    let aggregation = chart["aggregation"].as_str().unwrap_or("sum");
    if !matches!(aggregation, "sum" | "average" | "count" | "min" | "max") {
        return Err(Error::Unsupported("chart aggregation".into()));
    }
    let sort = chart["sort"].as_str().unwrap_or("source");
    if !matches!(sort, "source" | "ascending" | "descending") {
        return Err(Error::InvalidReport("invalid chart sorting".into()));
    }
    let limit = chart["limit"].as_u64().unwrap_or(12);
    if !(1..=200).contains(&limit) {
        return Err(Error::InvalidReport("invalid chart category limit".into()));
    }
    let table = relations
        .table(source)
        .ok_or_else(|| Error::InvalidData(format!("missing chart table: {source}")))?;
    if !table.columns().iter().any(|c| c.name == field) {
        return Err(Error::InvalidData(format!(
            "missing chart category {source}.{field}"
        )));
    }
    let value = chart["value"].as_str().unwrap_or("");
    if aggregation != "count"
        && !table.columns().iter().any(|c| {
            c.name == value && matches!(c.data_type, DataType::Integer | DataType::Decimal)
        })
    {
        return Err(Error::InvalidData(format!(
            "missing numeric chart value {source}.{value}"
        )));
    }
    let mut groups: Vec<Point> = Vec::new();
    for row in table.rows() {
        let key = table.field(row, field).unwrap().display();
        let number = if aggregation == "count" {
            1.0
        } else {
            match table.field(row, value).unwrap() {
                DataValue::Integer(n) => *n as f64,
                DataValue::Decimal(n) => *n,
                DataValue::Null => continue,
                _ => return Err(Error::InvalidData("chart value is not numeric".into())),
            }
        };
        if let Some(group) = groups.iter_mut().find(|g| g.label == key) {
            group.values.push(number)
        } else {
            groups.push(Point {
                label: key,
                values: vec![number],
            });
        }
    }
    let mut points = groups
        .into_iter()
        .map(|point| {
            let value = match aggregation {
                "count" => point.values.len() as f64,
                "average" => point.values.iter().sum::<f64>() / point.values.len() as f64,
                "min" => point.values.iter().copied().fold(f64::INFINITY, f64::min),
                "max" => point
                    .values
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max),
                _ => point.values.iter().sum(),
            };
            (point.label, value)
        })
        .collect::<Vec<_>>();
    if sort != "source" {
        points.sort_by(|a, b| {
            if sort == "ascending" {
                a.1.total_cmp(&b.1)
            } else {
                b.1.total_cmp(&a.1)
            }
        });
    }
    points.truncate(limit as usize);
    if points.is_empty() {
        return Err(Error::InvalidData("chart has no data".into()));
    }
    if points.iter().any(|(_, value)| !value.is_finite()) {
        return Err(Error::InvalidData("chart values are not finite".into()));
    }
    if width < 90.0 || height < 60.0 {
        return Err(Error::InvalidReport("chart is too small to display".into()));
    }
    let bg = rgb(chart["background"].as_str().unwrap_or("#ffffff"))?;
    let base = rgb(chart["color"].as_str().unwrap_or("#2563eb"))?;
    let custom = chart
        .get("categoryColors")
        .filter(|v| !v.is_null())
        .map(|v| {
            v.as_object()
                .ok_or_else(|| Error::InvalidReport("invalid chart colors".into()))
        })
        .transpose()?;
    let title = chart["title"].as_str().unwrap_or("");
    let show_legend = chart["showLegend"].as_bool().unwrap_or(false);
    let show_labels = chart["showLabels"].as_bool().unwrap_or(false);
    let show_grid = chart["showGrid"].as_bool().unwrap_or(false);
    let left = if kind == "bar" { x + 65.0 } else { x + 30.0 };
    let bottom = top - height + if show_legend { 35.0 } else { 24.0 };
    let upper = top - if title.is_empty() { 12.0 } else { 28.0 };
    let right = x + width - 12.0;
    if right <= left || upper <= bottom {
        return Err(Error::InvalidReport("chart content does not fit".into()));
    }
    let plot_w = right - left;
    let plot_h = upper - bottom;
    out.push_str(&format!("q {x:.3} {:.3} {width:.3} {height:.3} re W n {:.4} {:.4} {:.4} rg {x:.3} {:.3} {width:.3} {height:.3} re f\n",top-height,bg.0,bg.1,bg.2,top-height));
    if !title.is_empty() {
        label(out, title, x + 8.0, top - 4.0, width - 16.0, 18.0, 12.0)?;
    }
    if show_grid {
        for i in 0..=4 {
            let f = i as f64 / 4.0;
            if kind == "bar" {
                let at = left + f * plot_w;
                out.push_str(&format!(
                    "q 0.86 0.89 0.93 RG 0.5 w {at:.3} {bottom:.3} m {at:.3} {upper:.3} l S Q\n"
                ));
            } else {
                let at = bottom + f * plot_h;
                out.push_str(&format!(
                    "q 0.86 0.89 0.93 RG 0.5 w {left:.3} {at:.3} m {right:.3} {at:.3} l S Q\n"
                ));
            }
        }
    }
    let min = points.iter().map(|(_, v)| *v).fold(0.0, f64::min);
    let max = points.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    let span = if max == min { 1.0 } else { max - min };
    let zero = (0.0 - min) / span;
    for (index, (name, number)) in points.iter().enumerate() {
        let color = match custom.and_then(|m| m.get(name)) {
            Some(v) => rgb(v
                .as_str()
                .ok_or_else(|| Error::InvalidReport("invalid category color".into()))?)?,
            None => base,
        };
        let fraction = (*number - min) / span;
        let (bx, by, bw, bh) = if kind == "bar" {
            let step = plot_h / points.len() as f64;
            let start = left + zero * plot_w;
            let end = left + fraction * plot_w;
            (
                start.min(end),
                upper - (index + 1) as f64 * step + step * 0.15,
                (end - start).abs(),
                step * 0.7,
            )
        } else {
            let step = plot_w / points.len() as f64;
            let start = bottom + zero * plot_h;
            let end = bottom + fraction * plot_h;
            (
                left + index as f64 * step + step * 0.15,
                start.min(end),
                step * 0.7,
                (end - start).abs(),
            )
        };
        out.push_str(&format!(
            "q {:.4} {:.4} {:.4} rg {bx:.3} {by:.3} {bw:.3} {bh:.3} re f Q\n",
            color.0, color.1, color.2
        ));
        if kind == "bar" {
            label(out, name, x + 2.0, by + bh, 60.0, bh.max(9.0), 8.0)?;
        } else {
            label(out, name, bx, bottom - 4.0, bw.max(12.0), 12.0, 8.0)?;
        }
        if show_labels {
            let text = number.to_string();
            label(out, &text, bx, by + bh, bw.max(20.0), 10.0, 8.0)?;
        }
    }
    if show_legend {
        let legend = if aggregation == "count" {
            "Count"
        } else {
            value
        };
        label(
            out,
            legend,
            x + 8.0,
            top - height + 14.0,
            width - 16.0,
            12.0,
            8.0,
        )?;
    }
    out.push_str("Q\n");
    Ok(())
}
