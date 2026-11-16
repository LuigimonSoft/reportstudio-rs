mod support;
use reportstudio::Error;
use serde_json::json;
use support::*;
#[test]
fn filled_box_and_line_emit_pdf_graphics_with_expected_geometry() {
    let mut f = filled();
    let items = f.report["report"]["header"]["elements"]
        .as_array_mut()
        .unwrap();
    items.push(
        json!({"type":"box","x":4,"y":0.1,"width":1,"height":0.2,"style":{"color":"#ff0000"}}),
    );
    items.push(
        json!({"type":"line","x":5.1,"y":0.1,"width":1,"height":0.02,"style":{"color":"#0000ff"}}),
    );
    let pdf = f.generate().unwrap();
    let pdf = String::from_utf8_lossy(&pdf);
    assert!(pdf.contains("1.0000 0.0000 0.0000 rg"));
    assert!(pdf.contains(" re f Q"));
    assert!(pdf.contains("0.0000 0.0000 1.0000 RG"));
    assert!(pdf.contains(" l S Q"));
}
#[test]
fn invalid_color_and_oversized_border_are_rejected() {
    let mut f = filled();
    f.report["report"]["header"]["elements"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"box","x":4,"y":0.1,"width":1,"height":0.2,"style":{"color":"red"}}));
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
    let element = &mut f.report["report"]["header"]["elements"][1];
    element["style"]["color"] = json!("#ff0000");
    element["borderWidth"] = json!(100);
    assert!(matches!(f.generate(), Err(Error::InvalidReport(_))));
}
