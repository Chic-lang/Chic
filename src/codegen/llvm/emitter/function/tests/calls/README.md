# Calls harness layout

- `direct.rs` exercises direct and function-pointer call lowering.
- `virtual.rs` covers trait/vtable dispatch paths, including receiver validation and slot metadata.
- `intrinsic.rs` drives startup/runtime call sites and marshaling.
- `fixtures.rs` provides the shared `emit_result` helper so tests only assemble MIR fixtures.

Add new call scenarios by extending the closest module and building a small MIR fixture alongside the test. Run `cargo test calls::` for a fast loop. Refresh coverage with `cargo llvm-cov --json --output-path coverage.json` so refactor2.md stays in sync with the harness.
