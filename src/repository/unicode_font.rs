//! Embedded Noto Sans JP font and metrics. The accompanying OFL license is in assets/fonts.
use std::sync::OnceLock;
static FONT: &[u8] = include_bytes!("../../assets/fonts/NotoSansJP-Regular.ttf");
static FACE: OnceLock<ttf_parser::Face<'static>> = OnceLock::new();
pub(crate) fn bytes() -> &'static [u8] {
    FONT
}
pub(crate) fn face() -> &'static ttf_parser::Face<'static> {
    FACE.get_or_init(|| ttf_parser::Face::parse(FONT, 0).expect("embedded font must be valid"))
}
pub(crate) fn supports(ch: char) -> bool {
    (ch as u32) <= 0xffff && face().glyph_index(ch).is_some()
}
pub(crate) fn width(text: &str, size: f64) -> f64 {
    let face = face();
    let advance: u64 = text
        .chars()
        .map(|ch| {
            face.glyph_index(ch)
                .and_then(|id| face.glyph_hor_advance(id))
                .unwrap_or(0) as u64
        })
        .sum();
    advance as f64 * size / f64::from(face.units_per_em())
}
