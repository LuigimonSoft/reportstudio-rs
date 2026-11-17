//! Pagination of complete bands. A band is never split between pages.
use super::format::FormatContext;
use super::layout::{band, check_unused, number};
use super::relations::{Relations, RowContext};
use crate::repository::{build_pdf_pages, Error, ReportData, Result};
use serde_json::Value;

pub(super) fn render(root: &Value, supplied: &ReportData) -> Result<Vec<u8>> {
    let data = super::parameters::apply(root, supplied)?;
    super::schema::validate(root, &data)?;
    let relations = Relations::new(root, &data)?;
    let report = &root["report"];
    let mut guess = 1;
    for _ in 0..8 {
        let pages = paginate(report, &data, &relations, guess)?;
        if pages.len() == guess {
            return Ok(build_pdf_pages(
                number(&report["page"], "width")? * 72.0,
                number(&report["page"], "height")? * 72.0,
                &pages,
            ));
        }
        guess = pages.len();
    }
    Err(Error::InvalidReport("page count did not converge".into()))
}

struct Pager<'a> {
    report: &'a Value,
    relations: &'a Relations<'a>,
    format: FormatContext<'a>,
    context: RowContext,
    count: usize,
    width: f64,
    height: f64,
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
    page_footer: Option<&'a Value>,
    page_footer_height: f64,
    page_footer_leading: f64,
    active_header: Option<(&'a Value, RowContext, Option<&'a str>)>,
    report_footer: Option<&'a Value>,
    report_footer_height: f64,
    pages: Vec<String>,
    y: f64,
}
impl Pager<'_> {
    fn context(&self, context: &RowContext) -> RowContext {
        let mut context = context.clone();
        context.page_number = self.pages.len();
        context.page_count = self.count;
        context
    }
    fn raw(
        &self,
        section: &Value,
        context: &RowContext,
        source: Option<&str>,
        compact: bool,
        first: bool,
        start: f64,
    ) -> Result<(f64, String)> {
        let mut y = start;
        let mut output = String::new();
        band(
            section,
            self.relations,
            self.format,
            &self.context(context),
            source,
            compact,
            first,
            &mut y,
            self.left,
            self.right,
            self.width,
            self.height,
            self.bottom,
            &mut output,
        )?;
        Ok((y, output))
    }
    fn start_page(&mut self) -> Result<()> {
        if self.pages.len() >= 10_000 {
            return Err(Error::InvalidReport("too many report pages".into()));
        }
        self.pages.push(String::new());
        self.y = self.top;
        if let Some(header) = self.report.get("pageHeader") {
            let (end, commands) = self.raw(header, &self.context, None, false, true, self.y)?;
            if end > self.limit() + 1e-9 {
                return Err(Error::Unsupported(
                    "page header and footer do not fit".into(),
                ));
            }
            self.y = end;
            self.pages.last_mut().unwrap().push_str(&commands);
        }
        if let Some((header, context, source)) = &self.active_header {
            let (end, commands) = self.raw(header, context, *source, false, true, self.y)?;
            if end > self.limit() + 1e-9 {
                return Err(Error::Unsupported(
                    "repeated detail header does not fit".into(),
                ));
            }
            self.y = end;
            self.pages.last_mut().unwrap().push_str(&commands);
        }
        Ok(())
    }
    fn limit(&self) -> f64 {
        self.height - self.bottom - self.page_footer_height + self.page_footer_leading
    }
    fn place(
        &mut self,
        section: &Value,
        context: &RowContext,
        source: Option<&str>,
        compact: bool,
        first: bool,
    ) -> Result<()> {
        let attempt = self.raw(section, context, source, compact, first, self.y);
        if let Ok((end, commands)) = &attempt {
            if *end <= self.limit() + 1e-9 {
                self.y = *end;
                self.pages.last_mut().unwrap().push_str(commands);
                return Ok(());
            }
        }
        if let Err(error) = attempt {
            if !matches!(&error, Error::Unsupported(message) if message == "multi-page or overflowing report")
            {
                return Err(error);
            }
        }
        self.start_page()?;
        let (end, commands) = self.raw(section, context, source, compact, first, self.y)?;
        if end > self.limit() + 1e-9 {
            return Err(Error::Unsupported("band does not fit on a page".into()));
        }
        self.y = end;
        self.pages.last_mut().unwrap().push_str(&commands);
        Ok(())
    }
}
fn paginate<'a>(
    report: &'a Value,
    data: &'a ReportData,
    relations: &'a Relations<'a>,
    count: usize,
) -> Result<Vec<String>> {
    let page = &report["page"];
    let margins = &page["margins"];
    let footer = report.get("footer").filter(|b| b["visible"] != false);
    let page_footer = report.get("pageFooter").filter(|b| b["visible"] != false);
    for section in [footer, page_footer].into_iter().flatten() {
        check_unused(section, "sections")?;
    }
    let footer_height = footer
        .map(|b| number(b, "height"))
        .transpose()?
        .unwrap_or(0.0);
    let page_height = page_footer
        .map(|b| number(b, "height"))
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
                .filter(|e| e["visible"] != false)
                .map(|e| number(e, "y"))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .fold(page_height, f64::min)
                .clamp(0.0, page_height)
        }
    } else {
        0.0
    };
    let default = relations.first_context()?;
    let mut p = Pager {
        report,
        relations,
        format: FormatContext {
            locale: report["locale"].as_str().unwrap_or("en-US"),
            currency: report["currency"].as_str().unwrap_or("USD"),
        },
        context: default.clone(),
        count,
        width: number(page, "width")?,
        height: number(page, "height")?,
        top: number(margins, "top")?,
        bottom: number(margins, "bottom")?,
        left: number(margins, "left")?,
        right: number(margins, "right")?,
        page_footer,
        page_footer_height: page_height,
        page_footer_leading: leading,
        active_header: None,
        report_footer: footer,
        report_footer_height: footer_height,
        pages: vec![String::new()],
        y: number(margins, "top")?,
    };
    if p.limit() <= p.top
        || p.height - p.bottom - p.page_footer_height + p.page_footer_leading
            - p.report_footer_height
            < p.top
    {
        return Err(Error::Unsupported(
            "page and report footers do not fit".into(),
        ));
    }
    if let Some(header) = report.get("header") {
        p.place(header, &default, None, false, true)?;
    }
    if let Some(header) = report.get("pageHeader") {
        p.place(header, &default, None, false, true)?;
    }
    for detail in report["details"]
        .as_array()
        .ok_or_else(|| Error::InvalidReport("missing details".into()))?
    {
        check_unused(detail, "groups")?;
        if let Some(field) = detail["groupBy"].as_str().filter(|s| !s.is_empty()) {
            let source = detail["dataset"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::InvalidReport("grouped detail requires a dataset".into()))?;
            let groups = grouped_rows(
                data,
                relations,
                source,
                field,
                detail["sortDirection"].as_str().unwrap_or("asc"),
            )?;
            let mut row_number = 0;
            if groups.is_empty() {
                check_unused(detail, "emptyMessage")?;
            }
            for contexts in groups {
                let indices = contexts
                    .iter()
                    .filter_map(|context| context.rows.get(source).copied())
                    .collect::<Vec<_>>();
                let mut first_context = contexts[0].clone();
                first_context.group_indices = Some(indices.clone());
                if let Some(header) = detail.get("groupHeader") {
                    // Keep a group heading with at least its first visible row.
                    let fits = p
                        .raw(header, &first_context, Some(source), false, true, p.y)
                        .ok()
                        .and_then(|(end, _)| {
                            p.raw(
                                &detail["band"],
                                &first_context,
                                Some(source),
                                true,
                                row_number == 0,
                                end,
                            )
                            .ok()
                        })
                        .is_some_and(|(end, _)| end <= p.limit() + 1e-9);
                    if !fits {
                        if p.y > p.top {
                            p.start_page()?;
                        }
                        let fits_empty = p
                            .raw(header, &first_context, Some(source), false, true, p.y)
                            .ok()
                            .and_then(|(end, _)| {
                                p.raw(
                                    &detail["band"],
                                    &first_context,
                                    Some(source),
                                    true,
                                    row_number == 0,
                                    end,
                                )
                                .ok()
                            })
                            .is_some_and(|(end, _)| end <= p.limit() + 1e-9);
                        if !fits_empty {
                            return Err(Error::Unsupported(
                                "group header and first row do not fit".into(),
                            ));
                        }
                    }
                    p.place(header, &first_context, Some(source), false, true)?;
                    p.active_header = Some((header, first_context.clone(), Some(source)));
                }
                for mut context in contexts {
                    row_number += 1;
                    context.row_number = Some(row_number);
                    context.group_indices = Some(indices.clone());
                    p.place(
                        &detail["band"],
                        &context,
                        Some(source),
                        true,
                        row_number == 1,
                    )?;
                }
                if let Some(footer) = detail.get("groupFooter") {
                    p.place(footer, &first_context, Some(source), false, true)?;
                }
                p.active_header = None;
            }
            continue;
        }
        if let Some(header) = detail.get("groupHeader") {
            p.place(header, &default, None, false, true)?;
            p.active_header = Some((header, default.clone(), None));
        }
        let source = detail["dataset"].as_str().unwrap_or("");
        if source.is_empty() {
            p.place(&detail["band"], &default, None, false, true)?;
        } else {
            let table = data
                .table(source)
                .ok_or_else(|| Error::InvalidData(format!("missing table: {source}")))?;
            let mut rows = 0;
            for index in 0..table.rows().len() {
                for mut context in relations.contexts(source, index)? {
                    rows += 1;
                    context.row_number = Some(rows);
                    p.place(&detail["band"], &context, Some(source), true, rows == 1)?;
                }
            }
            if rows == 0 {
                check_unused(detail, "emptyMessage")?;
            }
        }
        p.active_header = None;
        if let Some(footer) = detail.get("groupFooter") {
            p.place(
                footer,
                &default,
                (!source.is_empty()).then_some(source),
                false,
                true,
            )?;
        }
    }
    let final_limit =
        p.height - p.bottom - p.page_footer_height + p.page_footer_leading - p.report_footer_height;
    if p.y > final_limit + 1e-9 {
        p.start_page()?;
    }
    if p.y > final_limit + 1e-9 {
        return Err(Error::Unsupported("report footer does not fit".into()));
    }
    let total = p.pages.len();
    for index in 0..total {
        let mut context = default.clone();
        context.page_number = index + 1;
        context.page_count = count;
        let final_page = index + 1 == total;
        if let Some(section) = p.page_footer {
            let start = p.height
                - p.bottom
                - p.page_footer_height
                - if final_page {
                    p.report_footer_height
                } else {
                    0.0
                };
            let mut y = start;
            band(
                section,
                relations,
                p.format,
                &context,
                None,
                false,
                true,
                &mut y,
                p.left,
                p.right,
                p.width,
                p.height,
                p.bottom,
                &mut p.pages[index],
            )?;
        }
        if final_page {
            if let Some(section) = p.report_footer {
                let mut y = p.height - p.bottom - p.report_footer_height;
                band(
                    section,
                    relations,
                    p.format,
                    &context,
                    None,
                    false,
                    true,
                    &mut y,
                    p.left,
                    p.right,
                    p.width,
                    p.height,
                    p.bottom,
                    &mut p.pages[index],
                )?;
            }
        }
    }
    Ok(p.pages)
}

fn grouped_rows(
    data: &ReportData,
    relations: &Relations<'_>,
    source: &str,
    field: &str,
    direction: &str,
) -> Result<Vec<Vec<RowContext>>> {
    use crate::repository::DataValue;
    if !matches!(direction, "asc" | "desc") {
        return Err(Error::InvalidReport("invalid group sort direction".into()));
    }
    let table = data
        .table(source)
        .ok_or_else(|| Error::InvalidData(format!("missing table: {source}")))?;
    if !table.columns().iter().any(|column| column.name == field) {
        return Err(Error::InvalidData(format!(
            "unknown group field {source}.{field}"
        )));
    }
    let mut groups: Vec<(DataValue, Vec<RowContext>)> = Vec::new();
    for index in 0..table.rows().len() {
        for context in relations.contexts(source, index)? {
            let value = relations
                .value(&context, source, field)?
                .cloned()
                .unwrap_or(DataValue::Null);
            if let DataValue::Text(ref s) | DataValue::Date(ref s) = value {
                if !s.is_ascii() {
                    return Err(Error::Unsupported("locale-aware group ordering".into()));
                }
            }
            let key = if value == DataValue::Null {
                DataValue::Text(String::new())
            } else {
                value
            };
            if let Some((_, rows)) = groups.iter_mut().find(|(v, _)| *v == key) {
                rows.push(context);
            } else {
                groups.push((key, vec![context]));
            }
        }
    }
    groups.sort_by(|(a, _), (b, _)| {
        let order = match (a, b) {
            (DataValue::Integer(x), DataValue::Integer(y)) => x.cmp(y),
            (DataValue::Decimal(x), DataValue::Decimal(y)) => x.total_cmp(y),
            _ => {
                let x = a.display();
                let y = b.display();
                x.to_ascii_lowercase()
                    .cmp(&y.to_ascii_lowercase())
                    .then_with(|| y.cmp(&x))
            }
        };
        if direction == "desc" {
            order.reverse()
        } else {
            order
        }
    });
    Ok(groups.into_iter().map(|(_, contexts)| contexts).collect())
}
