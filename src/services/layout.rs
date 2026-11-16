use super::relations::{Relations, RowContext};
use crate::repository::{build_pdf, Error, ReportData, Result};
use serde_json::Value;

fn number(v: &Value, key: &str) -> Result<f64> {
    v.get(key)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite())
        .ok_or_else(|| Error::InvalidReport(format!("missing or invalid {key}")))
}
fn check_unused(value: &Value, key: &str) -> Result<()> {
    if value.get(key).is_some_and(|v| {
        !v.is_null()
            && v != false
            && v.as_array().is_none_or(|a| !a.is_empty())
            && v.as_str().is_none_or(|s| !s.is_empty())
    }) {
        return Err(Error::Unsupported(key.into()));
    }
    Ok(())
}
pub(super) fn render(root: &Value, data: &ReportData) -> Result<Vec<u8>> {
    if root["version"] != 1 || root.get("schemaVersion").is_some_and(|v| v != 1) {
        return Err(Error::Unsupported("report schema version".into()));
    }
    super::parameters::validate(root)?;
    super::schema::validate(root, data)?;
    let relations = Relations::new(root, data)?;
    let report = root
        .get("report")
        .ok_or_else(|| Error::InvalidReport("missing report".into()))?;
    check_unused(report, "formulas")?;
    let page = &report["page"];
    let width = number(page, "width")?;
    let height = number(page, "height")?;
    if !(1.0..=100.0).contains(&width) || !(1.0..=100.0).contains(&height) {
        return Err(Error::InvalidReport("invalid page size".into()));
    }
    if page
        .get("background")
        .and_then(Value::as_str)
        .is_some_and(|c| !c.eq_ignore_ascii_case("#ffffff"))
    {
        return Err(Error::Unsupported("page background".into()));
    }
    let margins = &page["margins"];
    let top = number(margins, "top")?;
    let bottom = number(margins, "bottom")?;
    let left = number(margins, "left")?;
    let right = number(margins, "right")?;
    if [top, bottom, left, right].iter().any(|v| *v < 0.0)
        || left + right >= width
        || top + bottom >= height
    {
        return Err(Error::InvalidReport("invalid page margins".into()));
    }
    let mut commands = String::new();
    let mut y = top;
    let default_context = relations.first_context()?;
    for name in ["header", "pageHeader"] {
        if let Some(section) = report.get(name) {
            band(
                section,
                &relations,
                &default_context,
                None,
                false,
                true,
                &mut y,
                left,
                width,
                height,
                bottom,
                &mut commands,
            )?;
        }
    }
    let details = report["details"]
        .as_array()
        .ok_or_else(|| Error::InvalidReport("missing details".into()))?;
    for detail in details {
        for key in ["groups", "groupBy"] {
            check_unused(detail, key)?;
        }
        if let Some(section) = detail.get("groupHeader") {
            band(
                section,
                &relations,
                &default_context,
                None,
                false,
                true,
                &mut y,
                left,
                width,
                height,
                bottom,
                &mut commands,
            )?;
        }
        let source = detail["dataset"].as_str().unwrap_or("");
        if source.is_empty() {
            band(
                &detail["band"],
                &relations,
                &default_context,
                None,
                false,
                true,
                &mut y,
                left,
                width,
                height,
                bottom,
                &mut commands,
            )?;
        } else {
            let table = data
                .table(source)
                .ok_or_else(|| Error::InvalidData(format!("missing table: {source}")))?;
            let mut rendered_rows = 0_usize;
            for index in 0..table.rows().len() {
                for context in relations.contexts(source, index)? {
                    rendered_rows += 1;
                    band(
                        &detail["band"],
                        &relations,
                        &context,
                        Some(source),
                        true,
                        rendered_rows == 1,
                        &mut y,
                        left,
                        width,
                        height,
                        bottom,
                        &mut commands,
                    )?;
                }
            }
            if rendered_rows == 0 {
                check_unused(detail, "emptyMessage")?;
            }
        }
        if let Some(section) = detail.get("groupFooter") {
            band(
                section,
                &relations,
                &default_context,
                None,
                false,
                true,
                &mut y,
                left,
                width,
                height,
                bottom,
                &mut commands,
            )?;
        }
    }
    for name in ["pageFooter", "footer"] {
        if let Some(section) = report.get(name) {
            if section["visible"] != false
                && section["elements"]
                    .as_array()
                    .is_some_and(|elements| !elements.is_empty())
            {
                return Err(Error::Unsupported(
                    "page and report footer placement".into(),
                ));
            }
            band(
                section,
                &relations,
                &default_context,
                None,
                false,
                true,
                &mut y,
                left,
                width,
                height,
                bottom,
                &mut commands,
            )?;
        }
    }
    Ok(build_pdf(width * 72.0, height * 72.0, &commands))
}
#[allow(clippy::too_many_arguments)]
fn band(
    b: &Value,
    relations: &Relations<'_>,
    context: &RowContext,
    table: Option<&str>,
    compact: bool,
    first_row: bool,
    y: &mut f64,
    left: f64,
    width: f64,
    height: f64,
    bottom: f64,
    out: &mut String,
) -> Result<()> {
    if b["visible"] == false {
        return Ok(());
    }
    for key in [
        "sections",
        "visibilityExpression",
        "background",
        "backgroundExpression",
        "pageBreakBefore",
    ] {
        check_unused(b, key)?;
    }
    let prepared = super::table::prepare(b, relations, context, table, compact, first_row)?;
    let declared_height = number(b, "height")?;
    let mut band_height = if compact { 0.15_f64 } else { declared_height };
    if compact {
        for element in &prepared.elements {
            if element["visible"] == false {
                continue;
            }
            let mut element_height = number(element, "height")?;
            if element["type"] == "text" {
                let raw = element["text"]
                    .as_str()
                    .ok_or_else(|| Error::InvalidReport("text is missing".into()))?;
                let resolved = resolve(raw, relations, context, table)?;
                element_height = super::text::required_height(
                    &resolved,
                    &element["style"],
                    number(element, "width")? * 72.0,
                    element_height * 72.0,
                )? / 72.0;
            }
            let gap = if prepared.has_tables { 0.0 } else { 0.05 };
            band_height = band_height.max(number(element, "y")? + element_height + gap);
        }
    }
    if declared_height < 0.0 || band_height < 0.0 || *y + band_height > height - bottom + 1e-9 {
        return Err(Error::Unsupported(
            "multi-page or overflowing report".into(),
        ));
    }
    for e in &prepared.elements {
        check_unused(e, "chart")?;
        let x = number(e, "x")?;
        let ey = number(e, "y")?;
        let ew = number(e, "width")?;
        let eh = number(e, "height")?;
        if x < 0.0
            || ey < 0.0
            || ew <= 0.0
            || eh <= 0.0
            || left + x + ew > width
            || ey + eh > band_height + 1e-9
        {
            return Err(Error::InvalidReport("text outside section or page".into()));
        }
        let style = &e["style"];
        let rgb = parse_color(
            style
                .get("color")
                .and_then(Value::as_str)
                .unwrap_or("#000000"),
        )?;
        let color = (
            f64::from(rgb.0) / 255.0,
            f64::from(rgb.1) / 255.0,
            f64::from(rgb.2) / 255.0,
        );
        let px = (left + x) * 72.0;
        let top = (height - *y - ey) * 72.0;
        match e["type"].as_str() {
            Some("box") => {
                let border = e.get("borderWidth").and_then(Value::as_f64).unwrap_or(0.0);
                if !border.is_finite() || border < 0.0 || border > ew * 72.0 || border > eh * 72.0 {
                    return Err(Error::InvalidReport("invalid box border width".into()));
                }
                let border_color = parse_color(
                    e.get("borderColor")
                        .and_then(Value::as_str)
                        .unwrap_or("#000000"),
                )?;
                let inset = border / 2.0;
                out.push_str(&format!("q {:.4} {:.4} {:.4} rg {:.4} {:.4} {:.4} RG {:.3} w {:.3} {:.3} {:.3} {:.3} re {} Q\n",color.0,color.1,color.2,f64::from(border_color.0)/255.0,f64::from(border_color.1)/255.0,f64::from(border_color.2)/255.0,border,px+inset,top-eh*72.0+inset,ew*72.0-border,eh*72.0-border,if border>0.0 {"B"} else {"f"}));
                continue;
            }
            Some("line") => {
                let mid = top - eh * 36.0;
                out.push_str(&format!(
                    "q {:.4} {:.4} {:.4} RG {:.3} w {:.3} {:.3} m {:.3} {:.3} l S Q\n",
                    color.0,
                    color.1,
                    color.2,
                    eh * 72.0,
                    px,
                    mid,
                    px + ew * 72.0,
                    mid
                ));
                continue;
            }
            Some("text") => {}
            _ => return Err(Error::Unsupported(format!("element type {}", e["type"]))),
        }
        let value = e["text"]
            .as_str()
            .ok_or_else(|| Error::InvalidReport("text is missing".into()))?;
        let rendered = resolve(value, relations, context, table)?;
        let effective_height =
            super::text::required_height(&rendered, style, ew * 72.0, eh * 72.0)?;
        if ey + effective_height / 72.0 > band_height + 1e-9 {
            return Err(Error::Unsupported(
                "wrapped text exceeds section height".into(),
            ));
        }
        super::text::render(
            super::text::TextBox {
                text: &rendered,
                style,
                x: px,
                top,
                width: ew * 72.0,
                height: effective_height,
                color,
            },
            out,
        )?;
    }
    *y += band_height;
    Ok(())
}
fn parse_color(color: &str) -> Result<(u8, u8, u8)> {
    if color.len() != 7
        || !color.starts_with('#')
        || !color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(Error::InvalidReport("invalid color".into()));
    }
    Ok((
        u8::from_str_radix(&color[1..3], 16).unwrap(),
        u8::from_str_radix(&color[3..5], 16).unwrap(),
        u8::from_str_radix(&color[5..7], 16).unwrap(),
    ))
}
pub(super) fn resolve(
    text: &str,
    relations: &Relations<'_>,
    context: &RowContext,
    current: Option<&str>,
) -> Result<String> {
    let mut output = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        output.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after
            .find(']')
            .ok_or_else(|| Error::InvalidReport("unclosed field token".into()))?;
        let key = &after[..end];
        let (table, field) = match key.split_once('.') {
            Some((table, field)) => (table, field),
            None => (
                current.ok_or_else(|| {
                    Error::Unsupported(format!("field outside detail row: {key}"))
                })?,
                key,
            ),
        };
        if let Some(value) = relations.value(context, table, field)? {
            output.push_str(&value.display());
        }
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    Ok(output)
}
