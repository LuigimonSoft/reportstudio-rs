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

The renderer accepts version 1 reports, WinAnsi text and text covered by the embedded Noto Sans JP font, lines, filled boxes, opaque embedded PNG images, data-bound horizontal and vertical bar charts, designer-expanded detail table cells, and fields from related tables. It supports Helvetica variants, text alignment and wrapping, schema validation, and equality joins (null keys do not match). Numeric, currency, percentage and date formatting supports `en-US` and `es-MX`; currencies supported are USD, MXN, EUR, CAD, GBP and JPY. Expressions support arithmetic, comparisons, boolean operators, named formulas, IF and SUM; other functions return an explicit error. Page headers and footers repeat across pages; report footers appear on the final page, and page counters resolve to the final page count. Bands must fit intact on a page. Sections can use fixed backgrounds, literal or boolean-field visibility, nested bands, and detail row numbers; nested footer bands and backgrounds on nested bands are not yet supported. Saved parameter values and defaults filter supplied tables using equality, substring and bounds operators; missing required parameters are reported. Unsupported locales and currencies, unsupported expression functions, transparent or non-PNG images, unexpanded table definitions, dynamic table backgrounds, oversized bands characters outside the embedded font and bold or italic text requiring that font return an error rather than knowingly producing incomplete output. Single-field `groupBy` reports support ascending or descending ordering, group headers, group-scoped sums, and pagination. Text group ordering currently accepts ASCII only; nested group definitions (`groups`) remain unsupported. Line, area, pie and donut charts and complete layout and PDF parity remain future work. Bar and column charts are vector-rendered and support aggregation, category colors, labels and grids; their appearance is not yet pixel-identical to the designer.

Run `cargo test` for the data validation and PDF generation tests. Run `cargo run --example generate` from the project root to generate `example.pdf` from the included simple report.

## Test suite

Run `cargo test` to execute the unit, integration, architecture and Cucumber contracts. The browser suite independently checks text extraction, geometry and rendered pixels using PDF.js. The passing contracts cover the documented features; they do not establish full compatibility for every report or guarantee that unsupported features work.

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

Executable Gherkin scenarios live in `tests/features/implemented` and run through the Rust Cucumber runner with `cargo test --test bdd` (also included in `cargo test`). Step definitions live in `tests/bdd.rs` and each scenario receives isolated state. Unit and integration contracts cover additional behavior.

`tests/features/pending` records proposed behavior for features such as multi-page AWB layout. Those scenarios are **not executed yet** and do not have step definitions; they are specifications, not passing tests. Promote each scenario to `implemented` only after adding real steps and verifying its behavior. `tests/layer_dependencies.rs` provides a lightweight source-level guard against direct references to non-adjacent layers; it is not a complete static analysis of every possible Rust import.


## PDF verification in a browser

The optional Playwright suite generates actual PDF files through the Rust library, opens them in Chromium using PDF.js, and checks page count and dimensions, extracted text, text coordinates and visible pixels. It also compares text positions against PDFs generated by the designer, including alignment, wrapping and accented characters. It also checks that changed table data changes the output and that an invalid PDF is rejected. The suite also compares the single-page AWB example, its text positions and a tolerance-based raster comparison against the designer. This does not establish complete visual equivalence for other reports. No golden screenshots are assumed to be portable across operating systems.

Install Node.js 22 or later and Chromium once, then run the browser tests:

```sh
cd tests/browser
npm ci
npx playwright install chromium
npm test
```

Run `npm run test:all` in the same directory to execute `cargo test --locked` (including Cucumber) followed by the browser suite. `cargo test` by itself does not run Playwright and does not require Node.js. The tests use a temporary loopback server and temporary files; they require permission to bind to `127.0.0.1`.

## Unicode font

The library embeds a static regular-weight instance of Noto Sans JP for supported characters outside WinAnsi. Its source font is distributed under the SIL Open Font License; see `assets/fonts/OFL-NotoSansJP.txt`. Such PDFs include the font and a Unicode text mapping, increasing their size by approximately 5.5 MB. Characters outside that font, non-BMP characters, and bold or italic text that requires this fallback are currently rejected explicitly. Complex script shaping and complete Unicode coverage remain future work. The static instance was produced from Google Fonts' Noto Sans JP variable font at weight 400 using FontTools.

## CI and releases

GitHub Actions runs formatting and Clippy checks on Linux, Rust unit, integration and Cucumber tests on Linux, macOS and Windows, and PDF.js/Playwright browser checks on Linux for pushes and pull requests. The same checks run before a release is packaged.

To publish a release:

1. Configure the repository secret `CARGO_REGISTRY_TOKEN` with a crates.io API token authorized to publish `reportstudio`. For a first publication, the token must be allowed to publish a new crate. The `crates-io` GitHub environment may optionally be protected with required reviewers; its environment secrets can also hold the token.
2. Update the version in `Cargo.toml` and `Cargo.lock`, commit all required source files, fonts, licenses, examples, JSON fixtures and designer reference PDFs, and push the commit. The version must be available on crates.io.
3. Create a Git tag matching the manifest version, such as `v0.1.0`, push it, and publish a GitHub Release for that tag. The workflow also accepts a tag without the `v` prefix. Publishing a draft release starts the workflow; saving a draft does not.
4. After all checks pass, Actions verifies the package, publishes it to crates.io, and attaches the `.crate` and its SHA-256 checksum to the GitHub Release. Existing release assets are left untouched if their bytes differ. A `.crate` contains source and assets for Cargo to compile; it is not a standalone executable.

The release workflow does not modify the version from the tag. This keeps the published package tied to the committed manifest and lockfile. To inspect a package locally, run `cargo package --locked --list` and `cargo publish --locked --dry-run`. The latter does not publish. Release and registry publication are separate jobs: inspect both job results if either fails. A rerun skips an already published crate only when its bytes match the validated package. Once a version is published, crates.io does not permit replacing its contents; publish a new version for changes.
