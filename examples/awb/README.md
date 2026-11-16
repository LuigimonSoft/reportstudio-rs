# Air waybill compatibility fixture

`awb-full-form-example.json` is the populated Report Studio AWB example. All data is fictional. `awb-reference.pdf` was generated from that JSON by the current Report Studio TypeScript engine with its PDF text measurement adapter. It is a reference output, not output from this Rust library.

The Rust engine now loads the related tables but rejects the example with `unsupported report feature: text value formatting`; therefore no Rust PDF or visual parity claim is available yet. To support the fixture, the native implementation still needs expressions and aggregates, repeated detail layout, form boxes and borders, text formatting and wrapping, footer placement, and reliable pagination. These capabilities must be validated against the reference and against changed records; hard-coded output would not establish compatibility.
