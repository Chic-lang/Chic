# Perf Reporting & Budgets

- Budgets are compared against measured CPU microseconds; `runtime_adapter/native/cost.rs` provides
  deterministic checks used by CLI/reporting.
- Budget exceedances are flagged with `exceeded=true` and a signed `delta_us`.
- CI can gate on these results via `chic perf` or downstream tooling; WASM backends reuse the same
  schema but rely on stubbed collectors.
