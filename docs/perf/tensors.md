# Tensor Lowering Benchmarks

Benchmarks in `benches/tensor_codegen.rs` track the cost of tensor allocations, views, and copies across backends. The goal is to keep lowering deterministic (no hidden allocations) and to surface layout/stride costs explicitly.

## Bench Groups

| Group | Description |
|-------|-------------|
| `tensors::alloc::*` | Allocates row-major tensors with varying alignments and memspaces to measure stack vs. heap placement overhead. |
| `tensors::view::*` | Materialises sliced/strided views and records pointer arithmetic/bounds-check cost. |
| `tensors::copy::*` | Copies between contiguous and strided layouts, preferring tuned intrinsics and falling back to loop nests. |

Run them with:

```bash
cargo bench --bench tensor_codegen
```

## Perf Artifacts

- Baseline metadata lives under `profiling/tensors/llvm.json` and `profiling/tensors/wasm.json`. Each entry records the benchmark ID, explicit allocation counts, alignment, and relevant perf counters for the backend.
- Bench runs write updated JSON sidecars into `profiling/` so regression tooling can diff results deterministically.
- The JSON schema is shared across backends: `{ "bench": "...", "allocs": { "explicit": n }, "align": n, "layout": "...", "counters": { "cycles": n, "mem_bytes": n } }`.
- Recent local runs are archived in `profiling/tensors/latest.txt` (command and measurements) so regressions can be compared without rerunning the suite immediately.

## CI Integration

`cargo xtask metrics --bench tensor_codegen` should consume the recorded baselines and fail the build if allocations or perf counters regress by more than the configured threshold. Keep the JSON sidecars up to date whenever lowering changes.
