# Readonly Array Views – Stage 1

Last updated: 2025-03-04

## Overview

Stage 1 delivers a zero-copy, borrow-checked readonly view over `Array<T>` buffers
without introducing an owned “frozen” array. Developers obtain views through the
standard-library façade, the borrow checker ensures the backing array cannot be
mutated while any view is live, and both LLVM and WASM backends treat the view as
a lightweight `{ ptr: *const T, len: usize, elem_size: usize, elem_align: usize }`
struct matching the existing `Span<T>` layout.

Key traits:

- **Surface APIs**
  - `Std.Collections.Array.AsReadOnlySpan<T>(in ArrayPtr array)` – static helper that
    rewrites to the span runtime intrinsic and packages the result into a
    `ReadOnlySpan<T>` value.
  - `Std.Collections.Array<T>.AsReadOnlySpan(in this)` – extension method that makes
    instance-call syntax (`array.AsReadOnlySpan()`) available once extensions are
    enabled.
  - `Std.Span.ReadOnlySpan<T>` continues to expose slicing and length helpers; no
    new runtime functions were required.
- **Borrow semantics**
  - Every call to `AsReadOnlySpan` synthesises a shared borrow of the array backing
    storage. Multiple readonly views may coexist.
  - Any attempt to take a unique borrow, mutate the array, or reassign it while a
    view is live now triggers the standard conflicting-borrow diagnostic.
  - Releasing the view (assignment, `StorageDead`, or drop) clears the synthetic
    loan so subsequent mutations are allowed.
- **Implementation notes**
  - MIR lowering reuses the existing `ReadOnlySpan<T>` layout; no codegen augments
    were necessary beyond the borrow checker changes.
  - `BorrowChecker::track_readonly_span_call` associates the synthetic borrow with
    the destination view, letting `StorageDead`/`Drop` release the loan via the new
    `remove_loans_for_view` path.
  - Standard-library helpers now materialise a `Std.Runtime.Collections.ValueConstPtr`
    by reading the array’s pointer/stride metadata and call
    `chic_rt_span_from_raw_const` directly, so there is no longer a
    container-specific runtime export.
- **Testing**
  - New unit tests in `mir::borrow::tests` exercise the synthetic-loan path,
    asserting that a unique borrow after `AsReadOnlySpan` is rejected and allowed
    once the view is dropped.
  - `BorrowState` tests cover the new `remove_loans_for_view` helper.
  - Runtime coverage now includes host-side execution tests in
    `tests/runtime_readonly_span.rs` that validate concurrent views, cross-thread
    sharing, and failure paths (out-of-bounds slicing / copies).
  - A WASM harness (`tests/wasm/runtime_readonly_span.rs`) is in place and marked
    `#[ignore]` until the runtime exposes the required intrinsics for the module
    runner.

Stage 2 (owned/frozen arrays backed by drop glue) remains future work and is
tracked separately (see tracking issues).

## Usage Example

```chic
import Std.Collections;

public void Log(in ArrayPtr buffer)
{
    var view = buffer.AsReadOnlySpan<int>();
    for (var i = 0u; i < view.Length; i += 1u)
    {
        // hypothetical Write takes ReadOnlySpan<int>
        Logger.Write(view.Slice(i, 1u));
    }
    // attempting to mutate `buffer` here results in a borrow conflict.
}
```

## Spec & Documentation Updates

- `SPEC.md` references the readonly view behaviour alongside the
  span section, clarifying that `Array.AsReadOnlySpan` performs a shared borrow and
  that mutation attempts while a view is live produce diagnostics.
- Samples highlight how multiple readonly views can coexist and how dropping the
  view (via scope exit or reassignment) restores the ability to mutate the array.

## Future Work (Stage 2 – Owned Views)

- Empower the runtime with element drop glue so a “frozen” array can own its
  buffer safely.
- Provide zero-copy conversions from `Vec<T>` into a frozen array and back.
- Extend the façade with `IntoReadOnlyArray`/`ToReadOnlyArray` once drop glue lands.
