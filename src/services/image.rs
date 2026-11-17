//! Decode bounded embedded PNG images and emit inline PDF image streams.
use crate::repository::{Error, Result};
use base64::Engine;
use std::io::Cursor;

pub(super) fn render(
    source: &str,
    x: f64,
    top: f64,
    width: f64,
    height: f64,
    out: &mut String,
) -> Result<()> {
    let encoded = source
        .strip_prefix("data:image/png;base64,")
        .ok_or_else(|| Error::Unsupported("image must be an embedded PNG".into()))?;
    if encoded.len() > 12_000_000 {
        return Err(Error::InvalidData("image exceeds size limit".into()));
    }
    let data = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| Error::InvalidData("invalid image base64".into()))?;
    let mut decoder = png::Decoder::new(Cursor::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    decoder.set_limits(png::Limits {
        bytes: 32 * 1024 * 1024,
    });
    let mut reader = decoder
        .read_info()
        .map_err(|e| Error::InvalidData(format!("invalid PNG: {e}")))?;
    let info = reader.info();
    if info.animation_control.is_some() {
        return Err(Error::Unsupported("animated PNG images".into()));
    }
    let pixels = (info.width as usize)
        .checked_mul(info.height as usize)
        .ok_or_else(|| Error::InvalidData("image dimensions overflow".into()))?;
    if pixels == 0 || pixels > 4_000_000 {
        return Err(Error::InvalidData("image dimensions exceed limit".into()));
    }
    let mut bytes = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut bytes)
        .map_err(|e| Error::InvalidData(format!("invalid PNG: {e}")))?;
    let raw = &bytes[..frame.buffer_size()];
    let channels = match frame.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        _ => return Err(Error::Unsupported("PNG color type".into())),
    };
    let mut rgb = String::with_capacity(pixels * 6);
    use std::fmt::Write;
    for pixel in raw.chunks_exact(channels) {
        let (r, g, b, alpha) = match channels {
            1 => (pixel[0], pixel[0], pixel[0], 255),
            2 => (pixel[0], pixel[0], pixel[0], pixel[1]),
            3 => (pixel[0], pixel[1], pixel[2], 255),
            _ => (pixel[0], pixel[1], pixel[2], pixel[3]),
        };
        if alpha != 255 {
            return Err(Error::Unsupported("transparent PNG images".into()));
        }
        write!(rgb, "{r:02X}{g:02X}{b:02X}").unwrap();
    }
    out.push_str(&format!("q {width:.3} 0 0 {height:.3} {x:.3} {:.3} cm\nBI /Subtype /Image /W {} /H {} /CS /RGB /BPC 8 /F /ASCIIHexDecode ID\n{rgb}>\nEI\nQ\n",top-height,frame.width,frame.height));
    Ok(())
}
