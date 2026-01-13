# NdArray and Linalg MVP

This note documents the initial `Std.NdArray`/`Std.Linalg` surface added for the scientific stack MVP.

## Types and layout
- Owning `NdArray<T>` wraps a `Vec<T>` buffer plus `NdShape { dims, strides, length }`. Layout is row-major; `Length` is the product of dimensions.
- Non-owning `NdView<T>`/`NdViewMut<T>` hold a base span, shape metadata, and an element offset. Views borrow the underlying storage; no hidden allocations are introduced when slicing or permuting.
- Strides are expressed in element counts. Broadcasted dimensions record stride `0` so a single element is reused.

## Construction and shape utilities
- Builders: `NdArray<T>.FromVec(ref VecPtr data, ReadOnlySpan<usize> shape)`, `FromSlice(ReadOnlySpan<T>, ReadOnlySpan<usize>)`, `Zeros(shape)`, `Filled(shape, value)`.
- Shapes and slices use plain `Vec<usize>`/`Vec<NdSlice>` helpers; `NdSlice(start, length)` slices each axis, `NdSlice.All(len)` spans a full dimension.
- Reshape requires contiguous storage and preserves the underlying buffer; transpose/permute reorder shape/stride metadata without copying.

## Operations
- Elementwise: `Add`, `Subtract`, `Multiply`, `Divide` accept scalars or broadcastable views. Broadcasting aligns trailing dimensions (NumPy-style); incompatible shapes raise an `InvalidOperationException`.
- Linalg: `Std.Linalg.Dot` (rank-1 inputs) and `Std.Linalg.MatMul` (rank-2 inputs) implement naive row-major kernels with deterministic ordering. Shape checks guard inner-dimension mismatches.

## Tests and parity
- Cross-backend exec test: `tests/std_ndarray_exec.rs` builds `tests/testdate/ndarray.cl` for LLVM and WASM, covering shape/stride invariants, slicing, broadcasting, reshape/transpose, elementwise ops, and dot/matmul correctness.
- Broadcasting diagnostics and reshape guards are exercised in the same program; mutable views confirm alias safety via shared buffers.

## Benchmarks
- Baseline benches live in `benches/ndarray.rs` (`ndarray_elementwise_add_*`, `ndarray_matmul_naive_32`). Registered via `cargo bench --bench ndarray` and `cargo xtask metrics --bench ndarray`.
- Results serve as scalar baselines; SIMD/GPU variants can add side-by-side benches while keeping scalar paths deterministic for WASM.

## Integration notes
- Uses existing typed-pointer/Span/Vec runtime surfaces; no new extern ABI is added.
- Broadcasting/stride math is deterministic; scalar fallbacks remain the single source of truth while SIMD/GPU kernels are layered on.
- Future work: public shape helpers, additional reductions (sum/mean), SIMD/GPU fast paths, and richer view mutation APIs.
