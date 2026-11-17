mod support;
use reportstudio::{DataTable, DataType};
use serde_json::json;
use support::*;

fn multipage() -> Fixture {
    let mut f = Fixture::new();
    let mut t = DataTable::new("details");
    t.add_column("name", DataType::Text).unwrap();
    for n in 0..100 {
        t.add_row(vec![format!("Item{n:03}").into()]).unwrap();
    }
    f.data.add_table(t).unwrap();
    f.report["report"]["pageHeader"] = json!({"id":"ph","height":0.3,"elements":[text("HEADER")]});
    f.report["report"]["pageFooter"] =
        json!({"id":"pf","height":0.3,"elements":[text("Page [PAGE_NUMBER] of [PAGE_COUNT]")]});
    f.report["report"]["footer"] = json!({"id":"rf","height":0.3,"elements":[text("REPORT END")]});
    f.report["report"]["details"][0]["groupHeader"] =
        json!({"id":"gh","height":0.3,"elements":[text("DETAIL HEADER")]});
    f
}
#[test]
fn pages_repeat_headers_and_footers_and_preserve_every_row_once() {
    let pdf = multipage().generate().unwrap();
    let source = String::from_utf8_lossy(&pdf);
    let pages = source.matches("/Type /Page ").count();
    assert!(pages >= 3);
    assert_eq!(source.matches("(HEADER)").count(), pages);
    assert_eq!(source.matches("(DETAIL HEADER)").count(), pages);
    assert_eq!(source.matches("(REPORT END)").count(), 1);
    for n in 1..=pages {
        assert_eq!(source.matches(&format!("(Page {n} of {pages})")).count(), 1);
    }
    for n in 0..100 {
        assert_eq!(source.matches(&format!("(Item{n:03})")).count(), 1);
    }
}
#[test]
fn a_band_larger_than_available_page_is_rejected() {
    let mut f = multipage();
    f.report["report"]["details"][0]["band"]["elements"][0]["height"] = json!(11.0);
    assert!(f.generate().is_err());
}
