//! Serialize PDF pages and optional embedded Unicode font resources.
use super::unicode_font;
use std::fmt::Write;

pub(crate) fn build(width: f64, height: f64, commands: &str) -> Vec<u8> {
    build_pages(width, height, &[commands.to_owned()])
}
fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut output = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
    output.extend_from_slice(data);
    output.extend_from_slice(b"\nendstream");
    output
}
pub(crate) fn build_pages(width: f64, height: f64, pages: &[String]) -> Vec<u8> {
    assert!(!pages.is_empty());
    let unicode = pages.iter().any(|page| page.contains("BT /FU "));
    let font_id = 7 + pages.len() * 2;
    let kids = (0..pages.len())
        .map(|i| format!("{} 0 R", 7 + i * 2))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects:Vec<Vec<u8>>=vec![
      b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
      format!("<< /Type /Pages /Kids [{kids}] /Count {} >>",pages.len()).into_bytes(),
      b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
      b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec(),
      b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Oblique /Encoding /WinAnsiEncoding >>".to_vec(),
      b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-BoldOblique /Encoding /WinAnsiEncoding >>".to_vec(),
    ];
    for (i, commands) in pages.iter().enumerate() {
        let content = 8 + i * 2;
        let extra = if unicode {
            format!("/FU {font_id} 0 R")
        } else {
            String::new()
        };
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width:.3} {height:.3}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R {extra} >> >> /Contents {content} 0 R >>").into_bytes());
        objects.push(stream("", commands.as_bytes()));
    }
    if unicode {
        let face = unicode_font::face();
        let units = f64::from(face.units_per_em());
        let mut glyph_map = vec![0_u8; 65536 * 2];
        let mut widths = String::new();
        for code in 0..=0xffffu32 {
            if let Some(ch) = char::from_u32(code) {
                if let Some(glyph) = face.glyph_index(ch) {
                    let bytes = glyph.0.to_be_bytes();
                    let offset = code as usize * 2;
                    glyph_map[offset..offset + 2].copy_from_slice(&bytes);
                    let advance = face.glyph_hor_advance(glyph).unwrap_or(0);
                    let width = (f64::from(advance) * 1000.0 / units).round() as i32;
                    write!(widths, "{code} [{width}] ").unwrap();
                }
            }
        }
        let f = font_id;
        objects.push(format!("<< /Type /Font /Subtype /Type0 /BaseFont /NotoSansJP-Regular /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>",f+1,f+5).into_bytes());
        objects.push(format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /NotoSansJP-Regular /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /CIDToGIDMap {} 0 R /DW 1000 /W [{widths}] >>",f+2,f+4).into_bytes());
        let bbox = face.global_bounding_box();
        let scale = |v: i16| (f64::from(v) * 1000.0 / units).round() as i32;
        objects.push(format!("<< /Type /FontDescriptor /FontName /NotoSansJP-Regular /Flags 4 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {} 0 R >>",scale(bbox.x_min),scale(bbox.y_min),scale(bbox.x_max),scale(bbox.y_max),scale(face.ascender()),scale(face.descender()),scale(face.capital_height().unwrap_or(face.ascender())),f+3).into_bytes());
        let bytes = unicode_font::bytes();
        objects.push(stream(&format!("/Length1 {}", bytes.len()), bytes));
        objects.push(stream("", &glyph_map));
        let cmap=b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /ReportStudioUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n1 beginbfrange\n<0000> <FFFF> <0000>\nendbfrange\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";
        objects.push(stream("", cmap));
    }
    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}
