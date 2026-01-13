# Type declaration grammar tests

- Modules live in `structs.rs`, `traits.rs`, `enums.rs`, and `unions.rs` with shared helpers in `helpers.rs`.
- Fixtures come from `src/frontend/parser/tests/fixtures` (e.g., `PIXEL_SOURCE`, `assert_pixel_union`) and inline samples in each test file.
- Use `parse_module_allowing_errors` when asserting diagnostics and `parse_ok`/`parse_fail` for success/error flows.
- Run with `cargo test --lib frontend::parser::tests::grammar::declarations::types::`.
- When adding new cases, keep fixtures minimal and prefer module-specific files (struct-specific diagnostics vs. enum discriminant errors) to keep coverage focused.
