# Quantized Runtime Kernels

Quantized math now ships as Chic-native kernels under `Std.Linalg.Quantized`—no Rust stubs or adapter indirections on the critical path. The surface mirrors the MIR policy metadata (scales, zero-points, rounding, saturation) and executes deterministically on both LLVM and WASM.

## Kernel Surface (Std)

- `QuantizationPolicy` encodes scales/zero-points (per-tensor or per-channel via repeating arrays), `Bits`, `Signed`, `Saturate`, and `QuantizedRoundingMode` (`NearestEven` default, `TowardZero` supported). Bits are validated (<=31 signed, <=32 unsigned); zero scales are rejected deterministically.
- Scalar helpers: `Kernels.QuantizeScalar`, `Kernels.DequantizeScalar`, and `Kernels.Requantize` (policy-to-policy cast).
- Element-wise ops: `Kernels.Add` and `Kernels.Multiply` dequantize inputs, combine, and re-quantize to the output policy.
- Reductions: `Kernels.Dot` (pairwise dequantize/multiply/accumulate then quantize) and `Kernels.Gemm` (naive m×k · k×n loop nest with per-output-channel quantization). Shapes are validated; mismatches raise `ArgumentException`.
- All kernels clamp to the target int range when `Saturate` is set; when unset, results are still narrowed deterministically to `int` to avoid UB.

## Backends

- **Native/LLVM + WASM:** identical Chic implementations; no external intrinsics required. Vendor-specific acceleration can be layered later via codegen once policies are plumbed, but the Std kernels remain the correctness baseline.

## Tests

- `tests/linalg/quantized.ch` exercises rounding modes, saturation boundaries, element-wise ops, dot/gemm, requantization, and per-channel scale/zero-point handling. These run under both LLVM and WASM to enforce bit-for-bit parity.

## Usage Notes

- Per-channel policies use modulo indexing over the provided scale/zero-point arrays; empty arrays default to scale=1.0/zero=0.
- Output-channel quantization in GEMM uses the column index as the channel selector, matching common “per-output-channel” layouts.
- Thread-safe and allocation-free for hot paths: callers provide all buffers (`Span`/`ReadOnlySpan`) explicitly.
