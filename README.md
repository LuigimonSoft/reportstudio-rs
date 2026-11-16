# reportstudio-rs

Native Rust library for generating PDF files from Report Studio report JSON and caller-supplied table data. This project is in its initial implementation stage; it does not yet render the full Report Studio format.

```rust,no_run
use reportstudio::{DataTable, DataType, ReportData, ReportGenerator};

let mut details = DataTable::new("details");
details.add_column("description", DataType::Text)?;
details.add_column("quantity", DataType::Integer)?;
details.add_row(vec!["Consulting".into(), 10.into()])?;
let mut data = ReportData::new();
data.add_table(details)?;

ReportGenerator::new("reports/invoice.json")
    .with_data(data)
    .save_pdf("output/invoice.pdf")?;
# Ok::<(), reportstudio::Error>(())
```

Use `to_pdf_bytes()` to obtain the PDF in memory. Table names are report dataset IDs, and columns are field names. Rows must match the declared column order and types. `DataValue::Null` is supported for all types; dates use `DataValue::Date("YYYY-MM-DD".into())`. The application supplies records; the library does not connect to a database.

## Current scope

The initial renderer accepts version 1 reports with a single page, WinAnsi text, lines, filled boxes, designer-expanded detail table cells, and fields from related tables. It supports Helvetica's regular, bold, italic and bold-italic variants, text alignment, multiline text and word wrapping, validates declared schemas, and performs equality joins (null keys do not match). Missing required parameters are reported; parameter filtering is not implemented. Unsupported value formatting, formulas, unexpanded table definitions, dynamic table backgrounds, nested sections, page overflow, characters outside WinAnsi, and populated page/report footers return an error rather than knowingly producing incomplete output. Complex examples including the invoice, AWB and charts are not yet supported. Full layout and PDF parity remain future work.

Run `cargo test` for the data validation and PDF generation tests. Run `cargo run --example generate` from the project root to generate `example.pdf` from the included simple report.

## Test suite

Run `cargo test` to execute the currently supported behavior and list pending contracts. Tests marked `#[ignore]` describe behavior that is required but not implemented; they are not passing tests. To run a particular pending contract while implementing it:

```sh
cargo test --test future_contract detail_resolves_parent_field_through_relationship -- --ignored
```

To see every outstanding contract, use `cargo test --test future_contract -- --ignored`; this command is expected to fail until those features are implemented. Remove an ignore marker when its behavior is complete and the test passes normally. The AWB fixture has an independent data-loading test and two pending output tests; no test currently establishes visual equivalence with the reference PDF.

The PDF text assertions currently inspect uncompressed literal content emitted by the initial writer. They must be replaced with a portable PDF text extractor when compression or embedded font encodings are introduced. The browser tests below use PDF.js for independent text extraction and basic geometry/raster checks. Full visual parity, clipping, font metrics, graphics, and AWB placement still require reference-based image comparisons; successful text assertions alone cannot prove those properties.

## Architecture

The library follows a downward dependency chain:

```text
application  →  services  →  repository  →  models
```

Each layer imports the immediately lower layer. Types needed by an upper layer are re-exported through that layer's direct dependency; the library root is only the public facade.

- `application` exposes `ReportGenerator` and translates calls into input and output models.
- `services` coordinates the use case and performs report layout and validation.
- `repository` reads the report, persists the resulting PDF, and serializes PDF objects. The `DocumentRepository` trait allows storage to be replaced and tested independently.
- `models` holds table and report domain models, errors, and the `input` and `output` models. No domain model depends on a higher layer.

`ReportService::new` accepts an implementation of `DocumentRepository`; `ReportService::default` uses files. Existing callers can continue using `ReportGenerator` unchanged. The layout engine remains intentionally incomplete as described above.

## Behavior-driven tests

Executable Gherkin scenarios live in `tests/features/implemented` and run through the Rust Cucumber runner with `cargo test --test bdd` (also included in `cargo test`). Step definitions live in `tests/bdd.rs` and each scenario receives isolated state. Unit and integration contracts remain in place, including the 21 ignored contracts for unfinished features.

`tests/features/pending` records proposed behavior for features such as AWB. Those scenarios are **not executed yet** and do not have step definitions; they are specifications, not passing tests. Promote each scenario to `implemented` only after adding real steps and verifying its behavior. `tests/layer_dependencies.rs` provides a lightweight source-level guard against direct references to non-adjacent layers; it is not a complete static analysis of every possible Rust import.


## PDF verification in a browser

The optional Playwright suite generates actual PDF files through the Rust library, opens them in Chromium using PDF.js, and checks page count and dimensions, extracted text, text coordinates and visible pixels. It also compares text positions against PDFs generated by the designer, including alignment, wrapping and accented characters. It also checks that changed table data changes the output and that an invalid PDF is rejected. This currently covers only the supported simple report, not AWB or complete visual equivalence. No golden screenshots are assumed to be portable across operating systems.

Install Node.js 22 or later and Chromium once, then run the browser tests:

```sh
cd tests/browser
npm ci
npx playwright install chromium
npm test
```

Run `npm run test:all` in the same directory to execute `cargo test --locked` (including Cucumber) followed by the browser suite. `cargo test` by itself does not run Playwright and does not require Node.js. The tests use a temporary loopback server and temporary files; they require permission to bind to `127.0.0.1`.
