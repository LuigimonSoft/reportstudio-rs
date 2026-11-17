mod support;
use reportstudio::Error;
use serde_json::json;
use support::*;

#[test]
fn page_and_report_footers_resolve_single_page_counters() {
    let mut f = filled();
    f.report["report"]["pageFooter"] = json!({"id":"page-footer","height":0.5,"elements":[text("Page [PAGE_NUMBER] of [PAGE_COUNT]")]});
    f.report["report"]["footer"] =
        json!({"id":"report-footer","height":0.4,"elements":[text("END OF REPORT")]});
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Page 1 of 1"));
    assert!(has_text(&pdf, "END OF REPORT"));
}
#[test]
fn invisible_footer_is_not_rendered_or_reserved() {
    let mut f = filled();
    f.report["report"]["footer"] =
        json!({"id":"hidden","height":9,"visible":false,"elements":[text("Hidden footer")]});
    let pdf = f.generate().unwrap();
    assert!(!has_text(&pdf, "Hidden footer"));
    assert!(has_text(&pdf, "Alpha"));
}
#[test]
fn oversized_footer_returns_error_without_overwriting_output() {
    let mut f = filled();
    f.report["report"]["footer"] = json!({"id":"huge","height":10.4,"elements":[text("Too tall")]});
    std::fs::write(f.output_path(), b"existing").unwrap();
    assert!(matches!(f.save(), Err(Error::Unsupported(_))));
    assert_eq!(std::fs::read(f.output_path()).unwrap(), b"existing");
}
#[test]
fn leading_blank_page_footer_space_can_be_used_for_details() {
    let mut f = filled();
    f.report["report"]["pageFooter"] = json!({"id":"page-footer","height":9.0,"elements":[{"id":"p","type":"text","x":0,"y":8.7,"width":3,"height":0.2,"text":"Page [PAGE_NUMBER]","style":{"fontsize":9}}]});
    let pdf = f.generate().unwrap();
    assert!(has_text(&pdf, "Alpha"));
    assert!(has_text(&pdf, "Page 1"));
}
