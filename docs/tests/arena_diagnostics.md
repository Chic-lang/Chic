# Arena Diagnostics Harness

The `src/typeck/arena/tests/diagnostics` tree is now split into themed suites that
share a table-driven harness. Each suite (interfaces, signatures, auto traits,
etc.) defines an array of `ArenaDiagnosticCase` entries and calls
`run_cases("suite_name", CASES)` so new diagnostics can be added without copy/paste.

## Adding a new case

1. Pick the appropriate themed module (for example `interfaces.rs` for parser
   invariants or `auto_traits.rs` for ThreadSafe/Shareable work).
2. Use one of the case constructors:
   - `ArenaDiagnosticCase::parsed` for parser + typeck strings.
   - `ArenaDiagnosticCase::lowered` when MIR lowering must run.
   - `ArenaDiagnosticCase::custom` for bespoke builders that need to call the
     fixture helpers (`ArenaDiagnosticFixture`) directly.
3. Describe expectations with the helper constructors on `Expectation`
   (e.g. `contains`, `lacks`, or `with` when you need both).
4. Run `cargo test --lib typeck::arena::tests::diagnostics -- --nocapture`
   to execute the full suite.

## Coverage

Every time the diagnostics suite changes, refresh the focused coverage artifact:

```bash
cargo llvm-cov --lib --json --output-path coverage/arena_diagnostics.json -- typeck::arena::tests::diagnostics::
```

The JSON report is used by CI to ensure each new helper or suite stays above the
85% line-coverage target.
