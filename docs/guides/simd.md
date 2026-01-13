# SIMD vectors in Chic

This guide explains how to use the language-defined SIMD vectors described in `SPEC.md` and how to keep builds portable across LLVM and WASM.

## Syntax recap

- `vector<T, N>` is a first-class type. `T` must be `bool`, `i8`/`u8`, `i16`/`u16`, `i32`/`u32`, `i64`/`u64`, `f32`, or `f64`.
- `N` is a positive const-evaluable integer. Widths must land on 64/128/256 bits; other widths emit `TYPE0702`.
- Literals use constructor form: `vector<float, 4>(1.0f, 2.0f, 3.0f, 4.0f)`. Masks come from comparisons and are typed as `vector<bool, N>`.

## Std.Simd surface

All runtime-visible behaviour flows through Chic, not Rust:

- Constructors: `vector<T, N>.Zero()`, `vector<T, N>.Splat(T)`, `vector<T, N>.FromElements(...)`.
- Loads/stores: `Std.Simd.Load(ReadOnlySpan<T> src, int offset, SimdAlignment align)` and `Store(Span<T> dst, int offset, SimdAlignment align)`. `align` selects aligned vs unaligned paths; both remain safe.
- Lane ops: arithmetic (`+`, `-`, `*`, `/` for floats), bitwise (integers + masks), comparisons (produce masks), and swizzles via `Std.Simd.Shuffle`.
- Reductions: `Sum`, `Min`, `Max`, `Any`, `All`, `Dot`, `HorizontalAdd`, and `Blend` (mask-select).

## Target feature gating

Use attributes to declare requirements:

- `@requires_isa("avx2")`, `@requires_isa("sse4.2")`, `@requires_isa("neon")`, `@requires_isa("sve2")`, `@requires_feature("wasm_simd")` gate a function/type to specific ISAs.
- `@requires_simd` forbids scalar fallbacks. If the target cannot satisfy the requirement, the compiler emits `TYPE0704 SIMD_BACKEND_UNAVAILABLE`.
- Without strict requirements, the compiler may scalarise to per-lane loops on targets lacking SIMD. Behaviour stays deterministic; only performance changes.

### Backend support matrix (bootstrap compiler)

| Target | Widths | Feature knob | Behaviour |
| --- | --- | --- | --- |
| x86_64 (SSE4.2 baseline) | 128-bit vectors | enabled by default | Native vectors; 256-bit vectors require `--cpu-isa avx2` or `@requires_isa("avx2")` |
| x86_64 (AVX2/AVX-512) | 128/256-bit vectors | `--cpu-isa avx2` / `--cpu-isa avx512*` | Native vectors for matching widths; scalar fallback otherwise |
| AArch64 (NEON) | 128-bit vectors | default | Native NEON vectors; 256-bit widths scalarise unless SVE is requested |
| AArch64 (SVE/SVE2) | 128/256-bit vectors | `--cpu-isa sve` / `sve2` | Backend uses the requested SVE vector length; masks remain byte-backed |
| WASM | n/a (bootstrap) | `--wasm-simd` or manifest feature `wasm_simd128` | SIMD lowering not yet implemented; the backend rejects `vector<T, N>` today. Use LLVM for SIMD until WASM SIMD lands. |

Future GPU/accelerator mappings will follow the same rule: explicit ISA attributes gate lowering, and scalar fallbacks stay deterministic.

## Examples

```chic
import Std;
import Std.Simd;

// Require AVX2: code will not compile on targets without it.
@requires_isa("avx2")
public vector<float, 8> AddVectors(vector<float, 8> a, vector<float, 8> b)
{
    return a + b;
}

// Portable: scalarises when SIMD is unavailable.
public float Dot(ReadOnlySpan<float> left, ReadOnlySpan<float> right)
{
    let lanes = vector<float, 4>.Load(left, 0, SimdAlignment.Unaligned);
    let lanes2 = vector<float, 4>.Load(right, 0, SimdAlignment.Unaligned);
    let prod = lanes * lanes2;
    return Std.Simd.Dot(prod);
}
```

## Diagnostics you should expect

- `TYPE0701 VECTOR_LANES_MUST_BE_CONST` — lane count is zero or not const-evaluable.
- `TYPE0702 VECTOR_WIDTH_UNSUPPORTED` — width is not 64/128/256 bits.
- `TYPE0703 VECTOR_SHUFFLE_OOB` — shuffle indices outside `[0, N)`.
- `TYPE0704 SIMD_BACKEND_UNAVAILABLE` — requested ISA/SIMD feature is missing or the target (WASM bootstrap) cannot lower vectors.
- `TYPE0705 VECTOR_ELEMENT_UNSUPPORTED` — element type is outside the supported set.
- Bootstrap note: LLVM lowers vectors directly; the WASM backend currently rejects vector code until `wasm_simd128` lowering and fallback are completed.

## Testing & CI guidance

- Parser/typeck coverage belongs in `src/frontend/parser` and `src/typeck` tests; include both success and failure cases with the diagnostics above.
- Codegen tests should exercise both native and scalarised lowering for LLVM and WASM, keeping NaN/±0.0 behaviour identical.
- CI must reject new dependencies on Rust runtime symbols; SIMD lowering relies solely on Std metadata and backend intrinsics.
