use cucumber::{given, then, when, World};
use reportstudio::{DataTable, DataType, Error, ReportData, ReportGenerator};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, World)]
struct ReportWorld {
    directory: PathBuf,
    data: ReportData,
    table: Option<DataTable>,
    result: Option<reportstudio::Result<Vec<u8>>>,
    invalid_row_rejected: bool,
}
impl Default for ReportWorld {
    fn default() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "reportstudio-bdd-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).expect("create scenario directory");
        Self {
            directory,
            data: ReportData::new(),
            table: None,
            result: None,
            invalid_row_rejected: false,
        }
    }
}
impl Drop for ReportWorld {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
impl ReportWorld {
    fn report_path(&self) -> PathBuf {
        self.directory.join("report.json")
    }
    fn pdf_path(&self) -> PathBuf {
        self.directory.join("report.pdf")
    }
}

#[given("a simple report definition")]
fn simple_report(world: &mut ReportWorld) {
    fs::write(
        world.report_path(),
        include_bytes!("../examples/simple-report.json"),
    )
    .unwrap();
}
#[given("a malformed report definition")]
fn malformed_report(world: &mut ReportWorld) {
    fs::write(world.report_path(), b"{ invalid").unwrap();
}
#[given("a details table with two records")]
fn two_records(world: &mut ReportWorld) {
    let mut table = DataTable::new("details");
    table.add_column("description", DataType::Text).unwrap();
    table.add_row(vec!["First shipment".into()]).unwrap();
    table.add_row(vec!["Second shipment".into()]).unwrap();
    world.data.add_table(table).unwrap();
}
#[given("a details table with one text column")]
fn one_column(world: &mut ReportWorld) {
    let mut table = DataTable::new("details");
    table.add_column("description", DataType::Text).unwrap();
    world.table = Some(table);
}
#[when("I generate the PDF")]
fn generate(world: &mut ReportWorld) {
    world.result = Some(
        ReportGenerator::new(world.report_path())
            .with_data(world.data.clone())
            .to_pdf_bytes(),
    );
}
#[when("I save the PDF")]
fn save(world: &mut ReportWorld) {
    let result = ReportGenerator::new(world.report_path())
        .with_data(world.data.clone())
        .save_pdf(world.pdf_path());
    world.result = Some(result.and_then(|()| fs::read(world.pdf_path()).map_err(Error::from)));
}
#[when("I add a row with the wrong type")]
fn wrong_row(world: &mut ReportWorld) {
    world.invalid_row_rejected = world
        .table
        .as_mut()
        .unwrap()
        .add_row(vec![42.into()])
        .is_err();
}
#[then("the PDF contains both detail records")]
#[then("the saved file contains both detail records")]
fn pdf_contains_rows(world: &mut ReportWorld) {
    let bytes = world
        .result
        .as_ref()
        .expect("generation was run")
        .as_ref()
        .expect("generation succeeded");
    assert!(bytes.starts_with(b"%PDF-"));
    for value in ["First shipment", "Second shipment"] {
        assert!(
            bytes
                .windows(value.len())
                .any(|window| window == value.as_bytes()),
            "missing {value}"
        );
    }
}
#[then("a missing data error is returned")]
fn missing_data(world: &mut ReportWorld) {
    assert!(matches!(world.result, Some(Err(Error::InvalidData(_)))));
}
#[then("a JSON error is returned")]
fn json_error(world: &mut ReportWorld) {
    assert!(matches!(world.result, Some(Err(Error::Json(_)))));
}
#[then("the row is rejected and the table remains empty")]
fn row_rejected(world: &mut ReportWorld) {
    assert!(world.invalid_row_rejected);
    assert!(world.table.as_ref().unwrap().rows().is_empty());
}
fn main() {
    // Finder can create AppleDouble ._*.feature files on external volumes.
    // Give Cucumber only real features, without modifying the user's files.
    let mut features: Vec<_> = fs::read_dir("tests/features/implemented")
        .expect("read implemented feature directory")
        .map(|entry| entry.expect("read feature entry").path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "feature")
                && path
                    .file_name()
                    .is_some_and(|name| !name.to_string_lossy().starts_with("._"))
        })
        .collect();
    features.sort();
    assert!(
        !features.is_empty(),
        "no implemented Gherkin features were found"
    );
    for feature in features {
        futures::executor::block_on(ReportWorld::run(feature));
    }
}

#[given("a report with a relationship to customers")]
fn related_report(world: &mut ReportWorld) {
    let mut report: serde_json::Value =
        serde_json::from_slice(include_bytes!("../examples/simple-report.json")).unwrap();
    report["relations"] = serde_json::json!([{"id":"customer-rel","from":{"dataset":"details","field":"customerId"},"to":{"dataset":"customers","field":"id"},"cardinality":"n:1"}]);
    report["report"]["details"][0]["band"]["elements"][0]["text"] =
        serde_json::json!("[customers.name]: [description]");
    fs::write(world.report_path(), serde_json::to_vec(&report).unwrap()).unwrap();
}
#[given("customer and detail records with matching keys")]
fn related_records(world: &mut ReportWorld) {
    let mut customer = DataTable::new("customers");
    customer.add_column("id", DataType::Integer).unwrap();
    customer.add_column("name", DataType::Text).unwrap();
    customer
        .add_row(vec![1.into(), "Example Customer".into()])
        .unwrap();
    let mut detail = DataTable::new("details");
    detail.add_column("customerId", DataType::Integer).unwrap();
    detail.add_column("description", DataType::Text).unwrap();
    detail.add_row(vec![1.into(), "Shipment".into()]).unwrap();
    world.data.add_table(customer).unwrap();
    world.data.add_table(detail).unwrap();
}
#[then("the PDF contains the matching customer and detail")]
fn joined_result(world: &mut ReportWorld) {
    let bytes = world
        .result
        .as_ref()
        .expect("generation was run")
        .as_ref()
        .expect("generation succeeded");
    let text = b"Example Customer: Shipment";
    assert!(bytes.windows(text.len()).any(|window| window == text));
}

#[given("a report with a multiline heading")]
fn multiline_heading(world: &mut ReportWorld) {
    let mut report: serde_json::Value =
        serde_json::from_slice(include_bytes!("../examples/simple-report.json")).unwrap();
    report["report"]["header"]["height"] = serde_json::json!(0.8);
    report["report"]["header"]["elements"][0]["height"] = serde_json::json!(0.6);
    report["report"]["header"]["elements"][0]["text"] =
        serde_json::json!("First line\nSecond line");
    fs::write(world.report_path(), serde_json::to_vec(&report).unwrap()).unwrap();
}
#[then("the PDF contains both lines of the heading")]
fn both_heading_lines(world: &mut ReportWorld) {
    let bytes = world
        .result
        .as_ref()
        .expect("generation was run")
        .as_ref()
        .expect("generation succeeded");
    for line in ["First line", "Second line"] {
        assert!(bytes
            .windows(line.len())
            .any(|window| window == line.as_bytes()));
    }
}

#[given("a report with a detail table")]
fn report_with_table(world: &mut ReportWorld) {
    fs::write(
        world.report_path(),
        include_bytes!("browser/fixtures/table-report.json"),
    )
    .unwrap();
}
