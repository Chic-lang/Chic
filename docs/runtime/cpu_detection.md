# CPU Feature Detection

The runtime exposes shared CPU/SIMD detection utilities under `src/support/cpu.rs`. The helpers
cache the detected feature set (SSE2, SSE4.2, AVX2, NEON) on first use and surface convenience
queries so subsystems can choose between SIMD-accelerated and portable implementations.

## Usage

```rust
use chic::support::cpu;

if cpu::has_sse42() {
    // Enable SSE4.2-accelerated scanning / formatting.
}

let features = cpu::features();
debug_assert!(features.has_byte_simd() || !cpu::has_sse2());
```

Detection runs once per process via `OnceLock`; repeated calls reuse the cached snapshot.
Compilation is gated by the `runtime-simd` crate feature (enabled by default). Building with
`--no-default-features` disables SIMD detection entirely and forces the portable paths.

## Testing and Overrides

- `cpu::override_for_testing(...)` (compiled under `cfg(test)` or when the `simd-test-hooks`
  feature is enabled) lets unit tests and manual QA inject a synthetic feature set without relying
  on host CPUID support. The guard restores the previous state on drop.
- Setting the process environment variable `CHIC_CPU_OVERRIDE` to `scalar`, `simd`, or a
  comma/plus-separated list of features (`sse2+sse4.2`, `avx2+neon`, …) forces the runtime to use
  that feature mix for the lifetime of the process. This is primarily intended for integration
  tests and CLI-driven demos; invalid values are ignored with a warning.
- Unit tests in `src/support/cpu.rs` cover caching behaviour and override interactions. Runtime
  string tests (`src/runtime/string/tests.rs`) exercise the FFI surface with SIMD disabled to
  ensure fallbacks stay correct.
- Decimal fast paths (`src/runtime/decimal.rs` and `Std.Numeric.Decimal.Fast`) use these overrides to
  guarantee deterministic SIMD/Scalar selection in unit tests, benchmarks, and integration harnesses.

For manual QA, rebuild the crate with `cargo test --features simd-test-hooks` and use
`cpu::override_for_testing` inside ad-hoc harnesses or `dbg!` probes to emulate specific CPUs.

## Integration

- Runtime string mutation helpers consult the cached snapshot inside `copy_bytes` to decide whether
  to invoke AVX2/SSE2/SSE4.2/NEON copy paths or fall back to `ptr::copy_nonoverlapping`.
- The detection logic is architecture-aware (`is_x86_feature_detected!`,
  `is_aarch64_feature_detected!`) and returns `false` on unsupported architectures.
