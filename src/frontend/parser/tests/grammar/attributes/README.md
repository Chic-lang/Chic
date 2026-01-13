# Attribute grammar tests

- Modules: `builtin.rs` (general/builtin attributes), `mmio.rs` (mmio/register), `parsing.rs` (parser argument splitting/diagnostics). Shared helpers live in `helpers.rs`.
- Fixtures rely on inline snippets plus `lex_tokens` from `tests::fixtures`.
- Run via `cargo test --lib frontend::parser::tests::grammar::attributes::`.
- To refresh snapshots/expectations, re-run the test suite (no generated files).
- Keep new cases small and family-scoped (e.g., mmio register access, vectorize decimal target, struct layout args).
