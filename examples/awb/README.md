# Air waybill example

The fictional AWB separates its report definition from its input data:

- `awb-template.json`: report design, schemas and relationships; every dataset has an empty `data` array.
- `awb-data.json`: fictional rows grouped by dataset ID under `tables`.
- `data_loader.rs`: example adapter that reads schemas, creates typed `DataTable` columns, then adds rows from the external JSON in schema order to `ReportData`.
- `../awb_fixture.rs`: calls the adapter and passes the result to `ReportGenerator`.

Run from the repository root:

```sh
cargo run --example awb_fixture -- ./awb.pdf
cargo run --example awb_fixture -- ./awb.pdf ./my-awb-data.json
```

Use the example data file as the complete schema for your data: include every table ID (use an empty array for tables without records), use matching field names and scalar types, and preserve parent/child key values. Missing fields are null. The adapter currently supports the text, number and boolean types used by this example and rejects unknown tables, fields and incompatible values. It is example code; the library itself accepts `ReportData` rather than automatically loading arbitrary JSON.

`awb-full-form-example.json` is the original populated compatibility fixture. `awb-reference.pdf` was generated from it by the Report Studio TypeScript engine and serves as a visual reference. Browser tests compare the Rust output with this reference and verify that changes in an external data file appear in the PDF. Raster comparison allows small rendering differences.
