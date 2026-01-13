# Text formatter pipeline

The text formatter now has three layers:

1. `format/pretty.rs` – string-formatting helpers (visibility, parameters, const declarators).
2. `format/stream.rs` – streaming writer that emits AST nodes with indentation and spacing.
3. `format.rs` – thin orchestrator exposing `write_module` for callers.

When extending the formatter:
- Put new string helpers in `pretty.rs` and keep `format.rs` free of heavy logic.
- Add structural emission to `stream.rs` so indentation and doc handling stay consistent.
- Add targeted tests under `src/codegen/text/format/tests` (or within `stream.rs`) and refresh coverage with `cargo llvm-cov --lib --json --output-path coverage/text_format_local.json -- codegen::text::format::stream::tests::`.
