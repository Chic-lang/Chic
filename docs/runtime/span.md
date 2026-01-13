## Chic Span Runtime

This note captures the design of Chic’s `Span<T>` and `ReadOnlySpan<T>` primitives. The
overall goal is to provide zero-allocation, bounds-checked windows over contiguous memory that the
compiler can reason about for lifetime safety and optimisation, while keeping Chic’s ownership
guarantees intact.

### Design Goals

- **Borrowed view, not ownership.** A span is a lightweight triple *(ptr, length, layout)* over an
  existing backing store.  It never owns storage; dropping a span does not free memory.
- **Predictable layout.** Both `Span<T>` and `ReadOnlySpan<T>` lower to a runtime struct shaped as
  `{ Value{Mut,Const}Ptr data; [u8; 16] _reserved; usize len; usize elem_size; usize elem_align }`,
  where `Value{Mut,Const}Ptr` carries `(Pointer, Size, Alignment)`. The 16-byte reserved block keeps
  the length/stride fields 16-byte aligned and leaves space for future metadata without changing the
  ABI. This mirrors the collection layout we expose for `Array<T>`/`Vec<T>` and keeps codegen uniform
  across backends.
- **Bounds and safety.** Every slicing and projection helper performs bounds checks in the runtime.
  The compiler still emits borrow-check diagnostics if a span would outlive the value it was taken
  from.
- **Interop first.** Spans bridge to arrays, vectors, strings, and raw pointers.  Read-only spans can
  always be obtained from mutable spans; the reverse requires an explicit copy.
- **Async/pinning aware.** When a span is taken across `await`, the borrow checker ensures the
  backing storage is pinned (e.g., via `Vec::reserve_pinned`) or stack-based to avoid moving memory
  that the span still references.

### Runtime Surface

`packages/runtime.native/src/span.cl` (Std.Runtime.Native) defines the canonical #[repr(C)] mirrors:

```rust
pub struct ChicSpan {
    pub data: ValueMutPtr, // (ptr, size, align) for the element type
    pub _reserved: [u8; 16], // ABI-reserved padding (keeps headers 16-byte aligned)
    pub len: usize,
    pub elem_size: usize,
    pub elem_align: usize,
}
pub struct ChicReadOnlySpan {
    pub data: ValueConstPtr, // (ptr, size, align) for the element type
    pub _reserved: [u8; 16], // ABI-reserved padding (keeps headers 16-byte aligned)
    pub len: usize,
    pub elem_size: usize,
    pub elem_align: usize,
}
```
The native layout on 64-bit targets is fixed at 64 bytes: `data` @ 0, `_reserved` @ 24, `len` @ 40,
`elem_size` @ 48, and `elem_align` @ 56. Tests assert this layout so the ABI remains stable.

and a small suite of helpers exported with `@extern("C")`:

- `chic_rt_span_from_raw(_mut|_const)` – create (read-only) spans from raw parts by passing the
  shared `Std.Runtime.Collections.ValueMutPtr` / `ValueConstPtr` handles (pointer + size + alignment).
  Element metadata is cached in the span representation so follow-on slicing does not need to
  recompute stride metadata. Every container bridge (Vec, Array, strings, stackalloc) now routes
  through these typed handles instead of bespoke pointer-based intrinsics.
- `chic_rt_span_slice(_readonly)` – bounds-checked slicing that returns a new span sharing the
  backing storage while copying the cached stride information.
- `chic_rt_span_to_readonly` – convert a mutable span into a read-only span without copying.
- `chic_rt_span_copy_to` – element-wise copy between compatible spans, used by stdlib helpers.
- `chic_rt_span_copy_to` is also invoked automatically when `Span<T>.StackAlloc` receives an
  existing span/read-only span, so stack scratch buffers can be initialised from existing data
  without an intermediate heap allocation.

The bootstrap shim module `src/runtime/span.rs` now only declares FFI bindings to the native runtime
symbols so host-side tests and tooling can call into the Chic implementation without keeping a
second implementation of the span algorithms.

### Compiler Integration

- `Ty::Span` and `Ty::ReadOnlySpan` are new MIR types.  They are treated as sequence types so
  indexing, `.Length/.Count`, and `foreach` desugar the same way as vectors.
- Type layouts mark spans as list-like structures with pointer index 0 and length index 1.  Codegen
  already knows how to project these fields when lowering `Rvalue::Len` or indexed element loads.
- Borrow checking recognises spans as non-owning borrows; they participate in the uniqueness checks
  for `ref`/`in` parameter modes and get the same lifetime restrictions as references.
- Stack allocation uses a new intrinsic rvalue that the LLVM/WASM emitters lower to an `alloca`
  (native) or `memory.grow` backed scratch buffer (WASM).  The builder wires the intrinsic behind the
  standard library façade so user code simply calls `Span.StackAlloc<T>(len)`.
- When `Span.StackAlloc` is invoked with a span/read-only span argument, the builder materialises the
  length as a `usize`, lowers the allocation intrinsic, and emits a direct call to
  `chic_rt_span_copy_to` so the destination buffer is populated immediately.  This applies to
  both LLVM and WASM backends and relies on the same typed-pointer metadata used by other span
  bridges.

### Standard Library Surface

- `Span<T>.FromArray`, `Span<T>.FromVec`, `Span<T>.FromValuePointer` (typed handles only).
- `Span<T>.Slice(start, length)` plus the single-argument `Span<T>.Slice(start)` (mirrored on
  `ReadOnlySpan<T>`) – both bounds checked, returning new spans while delegating to the runtime for
  diagnostics.
- `Span<T>.StackAlloc(length)` and `Span<T>.StackAlloc(ReadOnlySpan<T>)` for allocation-free staging.
- `Span<T>.CopyTo(Span<T>)` and `Span<T>.CopyFrom(ReadOnlySpan<T>)`.
- `ReadOnlySpan<T>.CopyTo(Span<T>)` mirrors the mutable API minus mutating helpers and adds
  bridging from `string` and `str`.
- `Std.Collections.Array.AsSpan`/`AsReadOnlySpan` expose zero-copy bridges for `Array<T>`.
- `Std.Collections.Vec.AsSpan`/`AsReadOnlySpan` expose Vec-backed bridges (read-only bridges use
  `VecViewPtr` so multiple snapshots can coexist).
- `Std.Strings.string.AsUtf8Span()` exposes a `ReadOnlySpan<byte>` view over a UTF-8 string without
  allocating, `string.TryCopyUtf8(span, out written)` copies into user-provided buffers, and
  `Utf8String.FromSpan(ReadOnlySpan<byte>)` converts stack data back into `string` after validating
  that the span’s typed handle reports byte-sized, byte-aligned elements. Mutable callers should use
  `.AsReadOnly()` explicitly.
- `Std.Memory.StackAlloc` centralises allocation-free helpers so libraries that need typed buffers
  (numeric formatting, UTF-8 staging, runtime adapters) can ask for a stackalloc-backed `Span<T>` or
  `Value{Const,Mut}Ptr` without reintroducing raw pointer casts.

All helpers are implemented in Chic source and forward to the runtime externs.  The façade
ensures element size/alignment are computed once per call site, keeping runtime helpers generic.

### Allocation-Free Utilities

- `Span<T>.StackAlloc(length)` lowers to the dedicated MIR intrinsic.  LLVM emits an `alloca`, wraps
  the pointer/stride/alignment into a `ValueMutPtr`, and calls `chic_rt_span_from_raw_mut`
  while the WASM backend bumps the synthetic stack pointer and restores it in the epilogue so
  allocations remain scoped to the function.
- `Span<T>.StackAlloc(ReadOnlySpan<T>)` pairs the intrinsic with `CopyFrom`, keeping string/numeric
  helpers allocation-free by staging UTF-8 payloads directly on the stack.
- `ReadOnlySpan<T>.CopyTo` and `Span<T>.CopyFrom` are symmetrical helpers for allocation-free copies,
  which is now the recommended way to bridge between borrowed data and stack scratch buffers.
- `string.TryCopyUtf8(span, out written)` mirrors the span-centric workflow from the other direction
  so CLI utilities can decode command-line arguments without ever calling `string::from`.

### Best Practices

- Prefer `Span<T>.StackAlloc(len)` for short-lived buffers ≤ a few KB and copy into them with
  `ReadOnlySpan<T>.CopyTo` / `Span<T>.CopyFrom` rather than allocating `Vec<T>` or `string`.
- Use `string.AsUtf8Span()` to inspect existing text without allocating, and
  `string.TryCopyUtf8(dest, out written)` when you need a stable UTF-8 snapshot inside a caller
  supplied buffer; fall back to `Utf8String.FromSpan` (on a read-only span) only when you truly need
  an owned `string`.
- When copying between spans, use the typed-pointer façade (`Span<T>.CopyTo` / `CopyFrom`) instead
  of manual loops; these helpers validate stride/alignment and propagate runtime diagnostics.

### Borrow Checking & Async

- Taking a mutable span (`Span<T>`) from an `Array<T>` or `Vec<T>` produces a synthetic unique borrow
  on the backing collection.  Attempting to borrow the collection again while the span is live
  produces the standard "conflicting borrow" diagnostic.
- Read-only spans continue to synthesize shared borrows so mutation is prevented for the duration of
  each view.
- Spans captured across `await` now honour the existing pinning rules: unique borrows require the
  backing storage to be pinned (`@pin` locals, fixed Vec reservations, etc.).  The new async tests
  in `src/mir/borrow/tests/async/pinned.rs` cover both the error case (unpinned array) and the
  pinned success path.
- Stack-allocated spans behave like borrowed stack memory.  The borrow checker rejects `await`
  points that run while a stackalloc span is still live, emitting
  “cannot await while stack-allocated span `<name>` is live”.  Drop the span (or move it into a heap
  buffer) before yielding to keep async state machines sound; see
  `src/mir/borrow/tests/async/span.rs` for regression coverage.
- Derived spans created through slicing/read-only conversions keep the stackalloc metadata, so
  `await` points now fail even if only a slice of a stack-backed buffer is retained.  Calls to the
  runtime slicing intrinsics are traced back to their roots to enforce this consistently across
  LLVM/WASM lowering.

### Testing

- Runtime unit tests cover slicing edge cases, copy helpers, and mutation through spans that point
  at vectors/arrays.
- MIR builder tests confirm indexing and `.Length` on spans produce the expected projections and
  borrow checker metadata (`cargo test span_stack_alloc`).
- LLVM/WASM integration tests exercise span slicing, stack allocation, async pinning diagnostics,
  and bridging to strings (`src/codegen/wasm/tests/function_emitter/span.rs`,
  `tests/codegen_exec.rs::wasm_span_program_executes`).
- The CLI sample at `tests/testdate/span.cl` demonstrates stack allocation, string/UTF-8 bridging,
  and formatting in Chic code; `cargo bench --bench runtime_span_stackalloc -- --sample-size 10`
  tracks runtime copy throughput to keep regressions visible in CI.

### Future Work

- SIMD-aware span operations (vectorised `CopyTo`, fill, search) can be layered on top of the new
  abstractions.
- Once the optimiser grows escape analysis, we can elide bounds checks for provably safe slices in
  hot paths.
- Additional runtime hooks may be added for async pinning integration with executors once the async
  runtime matures.
