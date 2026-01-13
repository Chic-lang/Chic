# Runtime String SIMD Fast Paths

The runtime string subsystem utilises SIMD instructions (AVX2/SSE2/SSE4.2 on x86/x86_64, NEON on
AArch64) to accelerate common append operations. The SIMD path copies payload bytes in wide chunks
before falling back to `copy_nonoverlapping` for any remaining tail bytes.

## Feature Detection

Runtime detection lives in `src/support/cpu.rs` and is exposed via `support::cpu::{features,
has_sse2, has_sse42, has_neon}`. The helper caches the detected feature set and reports the
availability of SSE2, SSE4.2, AVX2, and NEON. The string implementation consults the snapshot inside
`copy_bytes`; no explicit configuration is required from callers under the default
`runtime-simd` feature.

To verify detection manually or inside QA harnesses:

```rust
use chic::support::cpu;

let features = cpu::features();
println!("SIMD detected: SSE2={} SSE4.2={} AVX2={} NEON={}",
    features.has_sse2(),
    features.has_sse42(),
    features.has_avx2(),
    features.has_neon(),
);

if cfg!(feature = "simd-test-hooks") {
    let _guard = cpu::override_for_testing(cpu::CpuFeatures::new(true, true, false, false));
    assert!(cpu::has_sse42());
}
```

## Testing

- Run `cargo test runtime::string` to validate inline and heap behaviours. Tests cover inline
  capacity handling and promotion to heap storage.
- Where host code interacts with runtime strings directly, ensure it uses
  `chic_rt_string_as_slice` rather than inspecting struct fields so inline/heap storage
  remains transparent.
- Retrieve error messages via `chic_rt_string_error_message(code)` to surface diagnostics
  that match the frontend literal parser.

## Benchmarks

Use Criterion to measure the SIMD-accelerated paths:

```sh
cargo bench --bench runtime_string_push
```

Mean timings observed on development hardware (AVX2-capable x86_64):

- `runtime_string_push_inline`: 19.93 ns
- `runtime_string_push_heap`: 57.12 ns
- `runtime_string_append_unsigned`: 38.88 ns

Benchmark artefacts are written beneath `target/criterion/runtime_string_*` for regression tracking.

## Fallback Behaviour

On CPUs without the required SIMD features the implementation falls back to `copy_nonoverlapping`.
No configuration changes are needed; the feature snapshot automatically selects the correct path.
