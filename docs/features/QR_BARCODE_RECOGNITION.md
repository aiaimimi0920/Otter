# OCR extension: QR codes and barcodes

Unlike [projection invitation QR codes](QR_PROJECTION.md), scanning codes on a pin uses an optional Loom OCR capability.
Loom owns capability packages, installation, authorization and decoded results. Otter dispatches commands
for the selected pin, mounts attachments and presents authorized Surfaces.
Supported symbologies, results and actions come from the currently enabled capability package,
not a built-in Otter decoding contract.

- When the catalog provides the binding and the selected image meets the enabled command's conditions
  and authorization requirements, `Ctrl+4` requests OCR; `Alt+4` toggles available OCR result layers.
  The host recognizes visibility for `neuro.official/ocr.codes.v1` and text OCR attachments.
  Actual coverage depends on the capability response; shortcuts neither install capabilities nor bypass authorization.
- Recognized content is data: it does not automatically open links or execute programs. Link commands
  and similar actions must use authorized host operations. Recognition, a trusted capability manifest,
  or a URL alone does not grant permission for external execution.
- Local capture and editing remain available without a matching capability.
  Importing a projection invitation is not a general-purpose code scanner.

For Otter changes, inspect [`extension_command_route.rs`](../../native/src/desktop/extension_command_route.rs)
and [`extension_visual.rs`](../../native/otter-media/src/extension_visual.rs), and run
`cargo test --locked --manifest-path native/Cargo.toml -p otter-media --lib extension_visual`.
Loom owns capability-package symbology and decoding regressions; Otter host tests do not replace them.
