//! Layout for the PDF standard Helvetica family, using the same metrics as the designer.
use super::helvetica_metrics::{BOLD_KERNING, BOLD_WIDTHS, REGULAR_KERNING, REGULAR_WIDTHS};
use crate::repository::{Error, Result};
use serde_json::Value;

pub(super) struct TextBox<'a> {
    pub text: &'a str,
    pub style: &'a Value,
    pub x: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub color: (f64, f64, f64),
}

fn win_ansi(ch: char) -> Option<u8> {
    match ch {
        ' '..='~' => Some(ch as u8),
        '\u{00a0}'..='\u{00ff}' => Some(ch as u8),
        '€' => Some(0x80),
        '‚' => Some(0x82),
        'ƒ' => Some(0x83),
        '„' => Some(0x84),
        '…' => Some(0x85),
        '†' => Some(0x86),
        '‡' => Some(0x87),
        'ˆ' => Some(0x88),
        '‰' => Some(0x89),
        'Š' => Some(0x8a),
        '‹' => Some(0x8b),
        'Œ' => Some(0x8c),
        'Ž' => Some(0x8e),
        '‘' => Some(0x91),
        '’' => Some(0x92),
        '“' => Some(0x93),
        '”' => Some(0x94),
        '•' => Some(0x95),
        '–' => Some(0x96),
        '—' => Some(0x97),
        '˜' => Some(0x98),
        '™' => Some(0x99),
        'š' => Some(0x9a),
        '›' => Some(0x9b),
        'œ' => Some(0x9c),
        'ž' => Some(0x9e),
        'Ÿ' => Some(0x9f),
        _ => None,
    }
}
fn measure(text: &str, size: f64, bold: bool) -> f64 {
    let widths = if bold { &BOLD_WIDTHS } else { &REGULAR_WIDTHS };
    let kerning = if bold { BOLD_KERNING } else { REGULAR_KERNING };
    // Unicode lines use the embedded font's metrics; WinAnsi keeps designer metrics.
    if text.chars().any(|ch| win_ansi(ch).is_none()) {
        return crate::repository::unicode_width(text, size);
    }
    let bytes: Vec<u8> = text
        .chars()
        .map(|ch| win_ansi(ch).expect("validated WinAnsi text"))
        .collect();
    let mut units = 0_i32;
    for (index, byte) in bytes.iter().copied().enumerate() {
        units += i32::from(widths[usize::from(byte - 32)]);
        if let Some(next) = bytes.get(index + 1) {
            if let Ok(index) = kerning.binary_search_by_key(&(byte, *next), |(a, b, _)| (*a, *b)) {
                units += i32::from(kerning[index].2);
            }
        }
    }
    f64::from(units) * size / 1000.0
}
fn wrap(text: &str, width: f64, size: f64, bold: bool, enabled: bool) -> Vec<String> {
    if !enabled {
        return text.split('\n').map(str::to_owned).collect();
    }
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if measure(&candidate, size, bold) <= width {
                current = candidate;
                continue;
            }
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            if measure(word, size, bold) <= width {
                current = word.to_owned();
                continue;
            }
            for ch in word.chars() {
                let candidate = format!("{current}{ch}");
                if !current.is_empty() && measure(&candidate, size, bold) > width {
                    lines.push(std::mem::take(&mut current));
                }
                current.push(ch);
            }
        }
        lines.push(current);
    }
    lines
}
fn layout_lines(text: &str, style: &Value, width: f64) -> Result<(f64, Vec<String>)> {
    let size = style["fontsize"]
        .as_f64()
        .filter(|n| n.is_finite() && (1.0..=100.0).contains(n))
        .ok_or_else(|| Error::InvalidReport("invalid font size".into()))?;
    let text = text.replace("\\n", "\n");
    if text.chars().any(|ch| {
        ch != '\n'
            && ch != '\t'
            && win_ansi(ch).is_none()
            && !crate::repository::unicode_supports(ch)
    }) {
        return Err(Error::Unsupported(
            "text is not covered by the embedded font".into(),
        ));
    }
    if text
        .chars()
        .any(|ch| ch != '\n' && ch != '\t' && win_ansi(ch).is_none())
        && text
            .chars()
            .any(|ch| ch != '\n' && ch != '\t' && !crate::repository::unicode_supports(ch))
    {
        return Err(Error::Unsupported(
            "text is not covered by the embedded font".into(),
        ));
    }
    let lines = wrap(
        &text.replace('\t', " "),
        width,
        size,
        style["bold"] == true,
        style["wrap"].as_bool().unwrap_or(true),
    );
    Ok((size, lines))
}
/// Required text box height in points after wrapping, matching the designer.
pub(super) fn required_height(text: &str, style: &Value, width: f64, height: f64) -> Result<f64> {
    let (size, lines) = layout_lines(text, style, width)?;
    Ok(height.max(lines.len() as f64 * size * 1.25))
}
fn escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        let byte = win_ansi(ch).expect("validated WinAnsi text");
        match byte {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(char::from(byte));
            }
            32..=126 => out.push(char::from(byte)),
            _ => out.push_str(&format!("\\{byte:03o}")),
        }
    }
    out
}

pub(super) fn render(spec: TextBox<'_>, output: &mut String) -> Result<()> {
    let style = spec.style;
    let (size, lines) = layout_lines(spec.text, style, spec.width)?;
    if style["format"]
        .as_str()
        .is_some_and(|s| !matches!(s, "text" | "number" | "currency" | "percent" | "date"))
    {
        return Err(Error::InvalidReport("invalid text value format".into()));
    }
    if style["fontFamily"].as_str().is_some_and(|s| {
        !matches!(
            s,
            "sans-serif" | "verdana" | "tahoma" | "trebuchet" | "segoe" | "system"
        )
    }) {
        return Err(Error::Unsupported("PDF font family".into()));
    }
    let alignment = style["align"].as_str().unwrap_or("left");
    if !matches!(alignment, "left" | "center" | "right") {
        return Err(Error::InvalidReport("invalid text alignment".into()));
    }
    let vertical = style["verticalAlign"].as_str().unwrap_or("top");
    if !matches!(vertical, "top" | "middle" | "bottom") {
        return Err(Error::InvalidReport(
            "invalid vertical text alignment".into(),
        ));
    }
    let bold = style["bold"] == true;
    let italic = style["italic"] == true;
    if (bold || italic)
        && lines
            .iter()
            .any(|line| line.chars().any(|ch| win_ansi(ch).is_none()))
    {
        return Err(Error::Unsupported("bold or italic Unicode font".into()));
    }
    let font = match (bold, italic) {
        (false, false) => "F1",
        (true, false) => "F2",
        (false, true) => "F3",
        (true, true) => "F4",
    };
    let line_height = size * 1.25;
    let free = (spec.height - lines.len() as f64 * line_height).max(0.0);
    let vertical_offset = match vertical {
        "middle" => free / 2.0,
        "bottom" => free,
        _ => 0.0,
    };
    output.push_str(&format!(
        "q {:.3} {:.3} {:.3} {:.3} re W n {:.4} {:.4} {:.4} rg\n",
        spec.x,
        spec.top - spec.height,
        spec.width,
        spec.height,
        spec.color.0,
        spec.color.1,
        spec.color.2
    ));
    for (index, line) in lines.iter().enumerate() {
        let line_width = measure(line, size, bold);
        let x = spec.x
            + match alignment {
                "center" => (spec.width - line_width) / 2.0,
                "right" => spec.width - line_width,
                _ => 0.0,
            };
        let baseline = spec.top - vertical_offset - size * 0.718 - index as f64 * line_height;
        if line.chars().any(|ch| win_ansi(ch).is_none()) {
            use std::fmt::Write;
            let mut encoded = String::new();
            for ch in line.chars() {
                write!(encoded, "{:04X}", ch as u32).unwrap();
            }
            output.push_str(&format!(
                "BT /FU {size:.3} Tf {x:.3} {baseline:.3} Td <{encoded}> Tj ET\n"
            ));
        } else {
            output.push_str(&format!(
                "BT /{font} {size:.3} Tf {x:.3} {baseline:.3} Td ({}) Tj ET\n",
                escape(line)
            ));
        }
        if style["underline"] == true && line_width > 0.0 {
            let thickness = (size / 16.0).max(0.5);
            let y = baseline - size * 0.12;
            output.push_str(&format!(
                "{:.4} {:.4} {:.4} RG {thickness:.3} w {x:.3} {y:.3} m {:.3} {y:.3} l S\n",
                spec.color.0,
                spec.color.1,
                spec.color.2,
                x + line_width
            ));
        }
    }
    output.push_str("Q\n");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::{escape, measure, win_ansi, wrap};
    #[test]
    fn standard_metrics_include_kerning() {
        assert!((measure("AV", 10.0, false) - 12.64).abs() < 0.0001);
        assert!((measure("AV", 10.0, true) - 13.09).abs() < 0.0001);
    }
    #[test]
    fn winansi_encodes_supported_characters_and_rejects_other_scripts() {
        assert_eq!(win_ansi('é'), Some(0xe9));
        assert_eq!(win_ansi('€'), Some(0x80));
        assert_eq!(win_ansi('東'), None);
        assert_eq!(escape("México · €"), r"M\351xico \267 \200");
    }
    #[test]
    fn wrapping_preserves_paragraphs_and_splits_long_words() {
        let lines = wrap("one two\nabcdefgh", 20.0, 10.0, false, true);
        assert_eq!(lines, vec!["one", "two", "abc", "defg", "h"]);
    }
}
