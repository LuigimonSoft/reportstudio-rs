#![allow(dead_code)]
use reportstudio::{DataTable, DataType, DataValue, ReportData, ReportGenerator};
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Fixture {
    dir: PathBuf,
    pub report: Value,
    pub data: ReportData,
}
impl Fixture {
    pub fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "reportstudio-contract-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self {
            dir,
            report: basic_report(),
            data: ReportData::new(),
        }
    }
    pub fn report_path(&self) -> PathBuf {
        self.dir.join("report.json")
    }
    pub fn output_path(&self) -> PathBuf {
        self.dir.join("output.pdf")
    }
    pub fn generate(&self) -> reportstudio::Result<Vec<u8>> {
        fs::write(
            self.report_path(),
            serde_json::to_vec(&self.report).unwrap(),
        )
        .unwrap();
        ReportGenerator::new(self.report_path())
            .with_data(self.data.clone())
            .to_pdf_bytes()
    }
    pub fn save(&self) -> reportstudio::Result<()> {
        fs::write(self.report_path(), self.report()).unwrap();
        ReportGenerator::new(self.report_path())
            .with_data(self.data.clone())
            .save_pdf(self.output_path())
    }
}
trait SerializeReport {
    fn report(&self) -> Vec<u8>;
}
impl SerializeReport for Fixture {
    fn report(&self) -> Vec<u8> {
        serde_json::to_vec(&self.report).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
pub fn basic_report() -> Value {
    json!({"version":1,"schemaVersion":1,"report":{"version":1,"id":"test","title":"Test","locale":"en-US","currency":"USD","page":{"width":8.5,"height":11,"margins":{"top":0.5,"right":0.5,"bottom":0.5,"left":0.5}},"header":{"id":"header","name":"Header","height":0.5,"elements":[text("Title")]},"details":[{"id":"detail","dataset":"details","band":{"id":"body","name":"Details","height":0.3,"elements":[text("[name]")]}}]},"datasets":[{"id":"details","name":"Details","schema":{"fields":[{"name":"name","type":"text"}]},"data":[]}]})
}
pub fn text(value: &str) -> Value {
    json!({"id":"text","type":"text","x":0,"y":0,"width":3,"height":0.2,"text":value,"src":"","style":{"fontsize":9,"bold":false,"italic":false,"align":"left","color":"#000000","wrap":false,"format":"text"}})
}
pub fn table(name: &str, rows: &[&str]) -> DataTable {
    let mut t = DataTable::new(name);
    t.add_column("name", DataType::Text).unwrap();
    for row in rows {
        t.add_row(vec![(*row).into()]).unwrap();
    }
    t
}
pub fn filled() -> Fixture {
    let mut f = Fixture::new();
    f.data
        .add_table(table("details", &["Alpha", "Beta"]))
        .unwrap();
    f
}
pub fn assert_pdf(bytes: &[u8]) {
    assert!(bytes.starts_with(b"%PDF-"), "missing PDF header");
    assert!(bytes.windows(5).any(|w| w == b"%%EOF"), "missing PDF EOF");
}
// The current PDF writer emits uncompressed literal strings. When the writer
// starts embedding fonts or compressing streams, replace this with a portable
// text-extraction oracle rather than weakening the semantic assertions.
pub fn has_text(bytes: &[u8], expected: &str) -> bool {
    bytes
        .windows(expected.len())
        .any(|s| s == expected.as_bytes())
}
pub fn sample_data(project: &Value) -> ReportData {
    let mut data = ReportData::new();
    for source in project["datasets"].as_array().unwrap() {
        let mut t = DataTable::new(source["id"].as_str().unwrap());
        let fields = source["schema"]["fields"].as_array().unwrap();
        for field in fields {
            let ty = match field["type"].as_str().unwrap() {
                "text" => DataType::Text,
                "number" => DataType::Decimal,
                "boolean" => DataType::Boolean,
                other => panic!("unexpected {other}"),
            };
            t.add_column(field["name"].as_str().unwrap(), ty).unwrap();
        }
        for row in source["data"].as_array().unwrap() {
            let values = fields
                .iter()
                .map(|field| {
                    let key = field["name"].as_str().unwrap();
                    match row.get(key) {
                        None | Some(Value::Null) => DataValue::Null,
                        Some(Value::String(v)) => DataValue::Text(v.clone()),
                        Some(Value::Number(v)) => DataValue::Decimal(v.as_f64().unwrap()),
                        Some(Value::Bool(v)) => DataValue::Boolean(*v),
                        v => panic!("invalid row value {v:?}"),
                    }
                })
                .collect();
            t.add_row(values).unwrap();
        }
        data.add_table(t).unwrap();
    }
    data
}
