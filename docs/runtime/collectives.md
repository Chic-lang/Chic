# Collectives Runtime

Collective communication hooks for deterministic profiling.

- Native adapter logs structured `CollectiveRecord` entries with sequence
  numbers, participant counts, byte counts, and a deterministic latency model
  (`runtime_adapter/native/collectives.rs`). `describe()` produces a stable
  string form for perf tooling.
- WASM adapter emits deterministic diagnostics and records the same structured
  entries with zero latency (`runtime_adapter/wasm/collectives.rs`); no network
  I/O is performed.
- Ordering is preserved by sequence number; no hidden communication occurs in
  the stub implementation.
- Metrics can be folded into perf snapshots alongside accelerator stream logs.

Tests: `tests/distributed_runtime.rs` ensures loopback messaging and collective stubs behave
predictably.
