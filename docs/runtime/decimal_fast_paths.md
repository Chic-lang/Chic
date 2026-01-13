# Decimal Fast Paths

The decimal runtime ships scalar and SIMD kernels for common aggregation
workloads. These helpers are consumed by `Std.Numeric.Decimal.Fast`, giving Chic
programs a lightweight way to accelerate hot loops without dropping into Rust
shims or handwritten assembly.

## Kernels

- `chic_rt_decimal_sum[_simd]` accumulates a span of `decimal` values,
  returning a `DecimalIntrinsicResult`. SIMD execution processes four elements
  at a time when AVX2/SSE/NEON support exists.
- `chic_rt_decimal_dot[_simd]` multiplies two spans element-wise and
  accumulates the products, signalling `InvalidOperand` if the spans differ in
  length.
- `chic_rt_decimal_matmul[_simd]` multiplies two dense matrices into a
  destination slice. Shape incompatibilities and null pointers are reported as
  `DecimalStatus` values.

All entry points accept typed pointer wrappers rather than raw integers:
`DecimalConstPtr` and `DecimalMutPtr` describe the source/destination buffers,
and `DecimalRoundingEncoding` carries the rounding discriminant. Chic code
constructs these wrappers via the span/raw-pointer helpers in
`Std.Numeric.Decimal`, while the runtime validates them before touching memory.

All kernels accept rounding encodings and a flags word. Setting
`DECIMAL_FLAG_VECTORIZE` enables the SIMD variant and is enforced at runtime so
callers cannot accidentally mix scalar/SIMD entry points.

## Dispatch and Overrides

`Std.Numeric.Decimal.Fast` chooses between scalar and SIMD kernels by combining the
caller's `DecimalVectorizeHint` with runtime feature detection from
`support::cpu`. Unit tests and benchmarks use
`cpu::override_for_testing(...)` (behind `cfg(test)`/`simd-test-hooks`) or the
`CHIC_CPU_OVERRIDE` environment variable to exercise both code paths
deterministically. Production callers can inspect
`DecimalIntrinsicResult.Variant` (or the `DecimalStatus` returned by `MatMul`)
to record which path executed; this is handy when you need to compare scalar
vs SIMD accuracy or trigger telemetry.

When the compiler spots manual loops that sum or dot decimal slices, it emits
DM0002 alongside a fix-it that rewrites the loop to `Std.Numeric.Decimal.Fast`. Running
`cargo lint --fix` applies the transformation automatically. The same workflow removes unused `@vectorize(decimal)` hints
reported by DM0001, keeping hot loops aligned with the runtime's dispatch
strategy.

## Benchmarks and CI

Criterion benchmarks live in `benches/decimal_fast.rs`. They run each kernel in
scalar mode (`CpuFeatures::none()`) and SIMD mode (all features enabled) to
measure the dispatch overhead. The CI job `cargo xtask metrics --bench
decimal_fast` captures these timings and compares them to
`coverage/metrics/bench/decimal_fast/decimal_fast_bench_baseline.json`. A
regression beyond 10 % (or 0.25 s) fails the build, keeping the Chic
implementations competitive with future SIMD optimisations.
