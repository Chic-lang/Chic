# LLVM Backend Notes

This document collects backend decisions that apply across LLVM lowering passes. Keep the sections focused on deterministic lowering rules and the invariants that tests enforce.

## Tensor Lowering

### MIR surface and metadata
- Tensor ops are represented in MIR as `TensorAlloc { place, element_ty, shape, layout, memspace, align }`, `TensorView { place, base, view_shape, view_stride, offset }`, `TensorCopy { dst, src }`, and `TensorFree { place }` (see `docs/mir/layout.md` and spec §16.1).
- Each tensor local is annotated with `ShapeMetadata { dims, symbolic_bounds }`, `LayoutMetadata { trait_id, stride }`, and `MemSpaceMetadata { trait_id }`. The metadata is serialised into `mir.json` so planners and backends can validate layout/stride usage.
- Views retain their own shape/stride pairs and a borrow to the owning tensor; drop scheduling ensures the owner outlives all active views and prevents hidden copies.

### Lowering strategy (LLVM)
- `TensorAlloc` materialises an explicit allocation: stack slots when the lifetime is lexical and fits the existing stack-allocation policy, heap allocations via the runtime allocator otherwise. Alignment comes from the MIR `align` field; when absent the backend rounds up to the target’s natural alignment for the element type and records the chosen value in metadata for deterministic replay.
- `TensorView` computes the derived pointer as `base + offset + Σ(index_i * stride_i)` using the view’s stride vector. All arithmetic uses the element width to stay byte-accurate; negative or zero-length strides are rejected with a diagnostic before codegen.
- `TensorCopy` prefers tuned intrinsics when the layout pair is recognised (contiguous row-major, aligned blocking, or vector-friendly strides). The fallback emits an explicit loop nest that walks the shape using the recorded strides, guaranteeing no hidden allocations and deterministic element order.
- `TensorFree` mirrors the allocation path: stack slots are reclaimed by the existing storage stack, heap allocations call the matching runtime free routine with the same alignment/memspace parameters used at alloc time.

### Alignment, layout, and diagnostics
- All allocations and pointer arithmetic honour the MIR-provided `layout` and `align` values; the backend must never insert extra padding or implicit re-packing. Stride vectors are trusted only after validation against the shape (no overflow, no overlapping writes for copies into the same tensor).
- Layout traits are treated as opaque IDs in codegen; backends may specialise known traits (e.g., `RowMajor`, `ColumnMajor`, `Blocked<N>`) but must emit a clear diagnostic when an unsupported trait reaches LLVM lowering.
- When a copy spans incompatible layouts without a tuned kernel, the backend emits a deterministic loop and records the slow path in perf metadata so planners can react. Hidden reallocations to “fix” layout are forbidden.

### Determinism and metadata
- Every tensor lowering records the chosen alignment, memspace, and layout handling in the emitted LLVM metadata stream so later passes and agent tooling can confirm bit-for-bit consistency across runs.
- Runtime calls are versioned and side-effect free beyond the explicit allocation; no allocator fallback or implicit zeroing is allowed unless requested by MIR (e.g., via `ZeroInit`).

### Regression coverage
- `tests/codegen/llvm/tensors.rs` exercises contiguous row-major copies (memcpy), strided fallback loop nests, sliced views with offset/stride metadata, explicit alignment/heap-vs-stack allocation policy, and layout mismatches surfacing diagnostics.

## Quantized Numerics

### MIR metadata
- Quantized values carry `QuantPolicy` metadata: policy kind (`PerTensor`, `PerChannel { axis }`, or custom trait ID), scale(s), zero point(s), rounding mode, and saturation flag. MIR rvalues encode the policy alongside operand types so lowering can stay deterministic.
- Operations such as `qgemm`, `qconv`, and `qcast` are represented as numeric intrinsics with attached policy metadata and channel axis hints. Unsupported policy combinations must emit diagnostics rather than silently widening.

### Lowering strategy (LLVM)
- Rounding is emitted explicitly using integer math: multiply by the reciprocal scale, add a policy-specific bias for ties, and truncate toward zero. `nearest_even` uses a bias/and sequence to guarantee bit-for-bit determinism.
- Saturation clamps results to the target bitwidth after rounding. The clamp uses signed/unsigned min/max pairs so behaviour matches the MIR policy even across backends.
- Per-channel scales/zero-points materialise as constant vectors; loop-based fallbacks index into those vectors so channel-wise arithmetic remains deterministic when vendor intrinsics are unavailable.
- Quantized GEMM/conv dispatch prefers vendor intrinsics when the policy matches a known kernel; otherwise the backend emits a tiled loop nest that applies scale/zero-point arithmetic per element.
- Policy metadata is threaded through to LLVM metadata so downstream tools can verify that the chosen lowering honours the MIR contract.

### Diagnostics and tests
- Unsupported rounding modes, mismatched channel axes, or mixed saturation/rounding combinations surface `Codegen` diagnostics instead of silently falling back.
- Regression coverage should live alongside other LLVM numerics in `tests/codegen/llvm` (quantized intrinsics, channel-wise arithmetic, and GEMM/conv fallbacks) to keep the lowering contract enforced.
- Current scaffolding lives in `tests/codegen/llvm/quantized.rs` to pin rounding, saturation, per-channel policies, and kernel selection behaviour.

## Accelerator Streams

### MIR surface
- `EnqueueKernel`, `EnqueueCopy`, `RecordEvent`, and `WaitEvent` carry `(stream, device, memspace)` metadata and deterministic `stream_id`s assigned by MIR lowering. Ownership rules enforce unique borrows for streams and buffers while work is in flight.

### Lowering strategy (LLVM)
- Stream submissions map to backend dispatch calls behind a driver abstraction (CUDA/HIP/Metal). The LLVM backend serialises submissions by default, respecting the MIR ordering; explicit dependencies via events are encoded so later backends can reorder only when allowed.
- Copies validate layout/memspace compatibility before lowering. Unsupported pairs emit diagnostics instead of inserting hidden staging buffers.
- Alignment requirements flow through to DMA calls; the backend never re-aligns silently. Events record the chosen alignment and layout so planners can reproduce transfers deterministically.

### Mock driver and tests
- A mock driver logs `enqueue`, `copy`, `record`, and `wait` operations in order so unit tests can assert deterministic sequencing without a real GPU. This ensures stream ownership and event ordering stay stable across refactors.
