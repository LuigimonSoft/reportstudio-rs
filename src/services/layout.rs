use super::relations::{Relations, RowContext};
use crate::repository::{build_pdf, Error, ReportData, Result};
use serde_json::Value;

pub(super) fn number(v: &Value, key: &str) -> Result<f64> {
    v.get(key)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite())
        .ok_or_else(|| Error::InvalidReport(format!("missing or invalid {key}")))
}
pub(super) fn check_unused(value: &Value, key: &str) -> Result<()> {
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
    match render_single(root, data) {
        Err(Error::Unsupported(reason))
            if reason == "multi-page or overflowing report" || reason == "groupBy" =>
        {
            super::pagination::render(root, data)
        }
        result => result,
    }
}
fn render_single(root: &Value, data: &ReportData) -> Result<Vec<u8>> {
    if root["version"] != 1 || root.get("schemaVersion").is_some_and(|v| v != 1) {
        return Err(Error::Unsupported("report schema version".into()));
    }
    let filtered = super::parameters::apply(root, data)?;
    let data = &filtered;
    super::schema::validate(root, data)?;
    let relations = Relations::new(root, data)?;
    let report = root
        .get("report")
        .ok_or_else(|| Error::InvalidReport("missing report".into()))?;
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
    let format = super::format::FormatContext {
        locale: report["locale"].as_str().unwrap_or("en-US"),
        currency: report["currency"].as_str().unwrap_or("USD"),
    };
    let mut commands = String::new();
    let mut y = top;
    let default_context = relations.first_context()?;
    for name in ["header", "pageHeader"] {
        if let Some(section) = report.get(name) {
            band(
                section,
                &relations,
                format,
                &default_context,
                None,
                false,
                true,
                &mut y,
                left,
                right,
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
                format,
                &default_context,
                None,
                false,
                true,
                &mut y,
                left,
                right,
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
                format,
                &default_context,
                None,
                false,
                true,
                &mut y,
                left,
                right,
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
                for mut context in relations.contexts(source, index)? {
                    rendered_rows += 1;
                    context.row_number = Some(rendered_rows);
                    band(
                        &detail["band"],
                        &relations,
                        format,
                        &context,
                        Some(source),
                        true,
                        rendered_rows == 1,
                        &mut y,
                        left,
                        right,
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
                format,
                &default_context,
                (!source.is_empty()).then_some(source),
                false,
                true,
                &mut y,
                left,
                right,
                width,
                height,
                bottom,
                &mut commands,
            )?;
        }
    }
    // On the final page the report footer ends at the bottom margin. The
    // page footer sits immediately above it; unused leading space remains
    // available for detail rows as in the designer.
    let footer = report
        .get("footer")
        .filter(|section| section["visible"] != false);
    let page_footer = report
        .get("pageFooter")
        .filter(|section| section["visible"] != false);
    for section in [footer, page_footer].into_iter().flatten() {
        check_unused(section, "sections")?;
    }
    let footer_height = footer
        .map(|section| number(section, "height"))
        .transpose()?
        .unwrap_or(0.0);
    let page_height = page_footer
        .map(|section| number(section, "height"))
        .transpose()?
        .unwrap_or(0.0);
    if footer_height < 0.0 || page_height < 0.0 {
        return Err(Error::InvalidReport("negative footer height".into()));
    }
    let leading = if let Some(section) = page_footer {
        let elements = section["elements"]
            .as_array()
            .ok_or_else(|| Error::InvalidReport("band has no elements array".into()))?;
        if section.get("background").is_some_and(|v| !v.is_null())
            || section
                .get("backgroundExpression")
                .is_some_and(|v| !v.is_null())
        {
            0.0
        } else {
            elements
                .iter()
                .filter(|element| element["visible"] != false)
                .map(|element| number(element, "y"))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .fold(page_height, f64::min)
                .clamp(0.0, page_height)
        }
    } else {
        0.0
    };
    let footer_start = height - bottom - footer_height;
    let page_footer_start = footer_start - page_height;
    if page_footer_start + leading < top || y > page_footer_start + leading + 1e-9 {
        return Err(Error::Unsupported(
            "multi-page or overflowing report".into(),
        ));
    }
    if let Some(section) = page_footer {
        let mut footer_y = page_footer_start;
        band(
            section,
            &relations,
            format,
            &default_context,
            None,
            false,
            true,
            &mut footer_y,
            left,
            right,
            width,
            height,
            bottom,
            &mut commands,
        )?;
    }
    if let Some(section) = footer {
        let mut footer_y = footer_start;
        band(
            section,
            &relations,
            format,
            &default_context,
            None,
            false,
            true,
            &mut footer_y,
            left,
            right,
            width,
            height,
            bottom,
            &mut commands,
        )?;
    }
    Ok(build_pdf(width * 72.0, height * 72.0, &commands))
}
#[allow(clippy::too_many_arguments)]
pub(super) fn band(
    b: &Value,
    relations: &Relations<'_>,
    format: super::format::FormatContext<'_>,
    context: &RowContext,
    table: Option<&str>,
    compact: bool,
    first_row: bool,
    y: &mut f64,
    left: f64,
    right: f64,
    width: f64,
    height: f64,
    bottom: f64,
    out: &mut String,
) -> Result<()> {
    if b["visible"] == false {
        return Ok(());
    }
    if let Some(expression) = b["visibilityExpression"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        match super::expressions::evaluate(expression, relations, context, table)? {
            crate::repository::DataValue::Boolean(true) => {}
            crate::repository::DataValue::Boolean(false) => return Ok(()),
            _ => {
                return Err(Error::InvalidReport(
                    "section visibility must return a boolean".into(),
                ))
            }
        }
    }
    check_unused(b, "pageBreakBefore")?;
    let prepared = super::table::prepare(b, relations, format, context, table, compact, first_row)?;
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
                let resolved = resolve(raw, relations, context, table, &element["style"], format)?;
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
    let background = if let Some(expression) = b["backgroundExpression"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        match super::expressions::evaluate(expression, relations, context, table)? {
            crate::repository::DataValue::Text(color) => Some(color),
            _ => {
                return Err(Error::InvalidReport(
                    "section background must return a color".into(),
                ))
            }
        }
    } else {
        b.get("background")
            .filter(|v| !v.is_null())
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| Error::InvalidReport("invalid section background".into()))
            })
            .transpose()?
    };
    if let Some(background) = background {
        let (r, g, blue) = parse_color(&background)?;
        out.push_str(&format!(
            "q {:.4} {:.4} {:.4} rg {:.3} {:.3} {:.3} {:.3} re f Q\n",
            f64::from(r) / 255.0,
            f64::from(g) / 255.0,
            f64::from(blue) / 255.0,
            left * 72.0,
            (height - *y - band_height) * 72.0,
            (width - left - right) * 72.0,
            band_height * 72.0
        ));
    }
    for e in &prepared.elements {
        if e["type"] != "chart" {
            check_unused(e, "chart")?;
        }
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
            Some("chart") => {
                super::chart::render(&e["chart"], relations, px, top, ew * 72.0, eh * 72.0, out)?;
                continue;
            }
            Some("image") => {
                let source = e["src"]
                    .as_str()
                    .ok_or_else(|| Error::InvalidReport("image source is missing".into()))?;
                super::image::render(source, px, top, ew * 72.0, eh * 72.0, out)?;
                continue;
            }
            Some("text") => {}
            _ => return Err(Error::Unsupported(format!("element type {}", e["type"]))),
        }
        let value = e["text"]
            .as_str()
            .ok_or_else(|| Error::InvalidReport("text is missing".into()))?;
        let rendered = resolve(value, relations, context, table, style, format)?;
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
    if let Some(children) = b.get("sections").filter(|v| !v.is_null()) {
        let children = children
            .as_array()
            .ok_or_else(|| Error::InvalidReport("sections must be an array".into()))?;
        if !children.is_empty()
            && (b.get("background").is_some_and(|v| !v.is_null())
                || b.get("backgroundExpression").is_some_and(|v| !v.is_null()))
        {
            return Err(Error::Unsupported("background on nested section".into()));
        }
        for child in children {
            band(
                child, relations, format, context, table, false, first_row, y, left, right, width,
                height, bottom, out,
            )?;
        }
    }
    Ok(())
}
pub(super) fn parse_color(color: &str) -> Result<(u8, u8, u8)> {
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
    style: &Value,
    format: super::format::FormatContext<'_>,
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
        if key == "ROW_NUMBER" {
            let number = context
                .row_number
                .ok_or_else(|| Error::Unsupported("ROW_NUMBER outside detail row".into()))?;
            output.push_str(&number.to_string());
            rest = &after[end + 1..];
            continue;
        }
        if matches!(key, "PAGE_NUMBER" | "PAGE_COUNT") {
            output.push_str(&super::format::value(
                &crate::repository::DataValue::Integer(if key == "PAGE_NUMBER" {
                    context.page_number
                } else {
                    context.page_count
                } as i64),
                style["format"].as_str().unwrap_or("text"),
                format,
            )?);
            rest = &after[end + 1..];
            continue;
        }
        let value = super::expressions::evaluate(key, relations, context, current)?;
        output.push_str(&super::format::value(
            &value,
            style["format"].as_str().unwrap_or("text"),
            format,
        )?);
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    Ok(output)
}
