# Tracing Runtime

Trace collectors emit `perf.json` snapshots deterministically.

- Native/WASM collectors (`runtime_adapter/{native,wasm}/tracing.rs`) record metrics with stable
  trace IDs/labels and bundle them into `PerfSnapshot` runs.
- Timestamps in tests use supplied CPU microsecond values; production collectors should use a
  deterministic clock when `CHIC_TRACE_FAKE_CLOCK=1`.
- Traces may embed run logs for RNG replay (see `docs/tooling/perf_json.md`).
- Collectors now always attach a `run_log` entry (empty when logging is disabled) so downstream
  replay tooling can rely on stable schema presence.
