# Air waybill compatibility fixture

`awb-full-form-example.json` is the populated Report Studio AWB example. All data is fictional. `awb-reference.pdf` was generated from that JSON by the current Report Studio TypeScript engine with its PDF text measurement adapter. It is a reference output, not output from this Rust library.

The one-page example can be generated with `cargo run --example awb_fixture -- examples/awb/awb-rust-example.pdf`. The example explicitly supplies the fictional table records to the library. Browser tests compare the extracted text, coordinates and raster output against the designer reference, and verify that changing an input record changes the PDF. Raster comparison allows small rendering differences; it does not establish universal pixel equality. Multi-page AWBs and other unsupported report features remain future work.
