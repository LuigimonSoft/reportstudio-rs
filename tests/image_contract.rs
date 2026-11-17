mod support;
use serde_json::json;
use support::*;
fn with_image(source: &str) -> Fixture {
    let mut f = filled();
    let mut image = text("");
    image["type"] = json!("image");
    image["src"] = json!(source);
    f.report["report"]["header"]["elements"]
        .as_array_mut()
        .unwrap()
        .push(image);
    f
}
#[test]
fn opaque_embedded_png_is_written_as_an_inline_image() {
    let f=with_image("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC");
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "/Subtype /Image"));
    assert!(has_text(&pdf, "FF0000>"));
}
#[test]
fn corrupted_png_is_rejected_without_overwriting_an_existing_file() {
    let f=with_image("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jCYQAAAAASUVORK5CYII=");
    std::fs::write(f.output_path(), b"keep").unwrap();
    assert!(matches!(f.save(), Err(reportstudio::Error::InvalidData(_))));
    assert_eq!(std::fs::read(f.output_path()).unwrap(), b"keep");
}
#[test]
fn unsupported_sources_do_not_silently_render_as_blank() {
    let f = with_image("https://example.com/image.png");
    assert!(matches!(
        f.generate(),
        Err(reportstudio::Error::Unsupported(_))
    ));
}
