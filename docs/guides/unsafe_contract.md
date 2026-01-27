# Unsafe Contract Guide

Chic’s optimiser, borrow checker, and runtime follow a shared aliasing
contract. This guide summarises the expectations placed on unsafe code and the
tooling that helps audit existing code bases.

## Pointer Qualifiers

Raw pointers opt into the contract via inline qualifiers. The following table
shows the low-level effect of each modifier:

| Qualifier | Meaning | Backend effect |
|-----------|---------|----------------|
| `@restrict` | Pointer is unique for the duration of the call | Emits `noalias + nocapture` attributes plus alias-scope metadata on loads/stores |
| `@noalias` | Explicit non-aliasing promise (but may still capture) | Adds `noalias` attributes to the parameter |
| `@readonly` / `@writeonly` | Restrict the access pattern for the pointee | LLVM receives `readonly`/`writeonly` attributes, borrow checking enforces the same rule |
| `@aligned(N)` | Promise that the pointer is aligned to `N` bytes | Propagated to LLVM/Wasm and the runtime intrinsics |
| `@expose_address` | Provenance may be erased (e.g. cast to integers) | Required for integer casts and exposes the pointer to host tooling |

Unsafe pointers that omit every qualifier now trigger a friendly warning
(`DM0211`). Annotating the type makes the aliasing intent explicit and allows
IDE tooling to provide inline hints.

Pointer/integer conversions must go through the sanctioned
`Std.Numeric.UIntPtr` helpers. `UIntPtr.FromPointer<T>` /
`.FromConstPointer<T>`/`.AsPointer<T>`/`.AsConstPointer<T>` round-trip raw
pointers through opaque handles, and the convenience wrappers
`UIntPtr.AddressOf<T>` / `.AddressOfConst<T>` expose the raw `nuint` address.
End-to-end coverage lives in
`tests/numeric_structs.rs` plus `mir::builder::tests::unsafe_pointers`.

## `Std.Memory.MaybeUninit<T>`

`MaybeUninit<T>` is the canonical way to work with partially-initialised
storage:

```chic
var slot = MaybeUninit<MyDropType>.Uninit();
slot.Write(value);
var ready = slot.AssumeInit();
```

Key properties:

* `dispose(ref this)` only drops the payload when it is live. The MIR drop
  lowering pass detects `MaybeUninit<T>` and skips automatic field drops, so
  there is no double-drop when the wrapper goes out of scope.
* `PushInitialized`, `PopInto`, `InsertInitialized`, `RemoveInto`, and
  `SwapRemoveInto` allow `Vec` users to move elements without extra copies.
* `AssumeInitRead`, `AssumeInitRef`, `ForgetInit`, and
  `MarkInitialized` can be composed to express richer placement patterns.

### Placement-friendly `Vec` APIs

The standard library now exposes placement wrappers on top of the existing
runtime intrinsics:

* `Vec.PushInitialized` / `Vec.PopInto` moves the payload between a local
  `MaybeUninit<T>` slot and the vector tail.
* `Vec.InsertInitialized`, `Vec.RemoveInto`, and `Vec.SwapRemoveInto` provide
  the same ergonomic guarantee for arbitrary positions.

These helpers compose with `MaybeUninit<T>` so containers never observe an
uninitialised element, and borrow checking can reason about the lifetime of the
payload.

### Zero-init intrinsics in practice

`Std.Memory.Intrinsics.ZeroInit`/`ZeroInitRaw` replace every pointer-cast-based
`memset` call in the standard library. The stubs live in `Std.Memory` purely so
bootstrap builds type-check; the compiler rewrites each call to MIR
`StatementKind::ZeroInit{,Raw}` statements before codegen. Examples:

```chic
unsafe
{
    // Zero initialises a managed out slot.
    Std.Memory.Intrinsics.ZeroInit(out slot);

    // Bulk-zeros an inline Vec buffer before handing it to user code.
    Std.Memory.Intrinsics.ZeroInitRaw(buffer, length * metadata.Size);
}
```

`MaybeUninit<T>.Write`, `.AssumeInit*`, and the placement helpers on `Vec`
declare `throws Std.InvalidOperationException`, making it obvious at the
call-site when logical errors (double writes, double reads, or uninitialised
slots) bubble up. The Vec façade routes its metadata through `usize`
sizes/alignments, so there is no longer an `(isize)` cast in front of the
runtime calls. See `tests/vec_placement.rs` and `tests/collections_facade.rs`
for end-to-end coverage across LLVM and Wasm builds.

## Tooling and Lints

* **Compiler hint**: unsafe pointer parameters without alias qualifiers emit
  `DM0211`, nudging developers to document the contract in-source.
* **Custom metadata**: the LLVM backend now decorates pointer loads/stores with
  alias scope metadata, and the Wasm backend publishes a
  `chx.alias.contracts` custom section so host tooling can audit call ABI
  metadata.
* **Debug asserts**: the runtime guards `chic_rt_memcpy` against
  overlapping ranges in debug builds so contract violations fail fast.

## Benchmark Notes

The clarified contract allows the backend to remove redundant copies. The
`VecPushPlacement` micro-benchmark (32-byte elements, single thread) shows the
impact of switching from value-based pushes to placement pushes:

| Variant | Throughput (items/ns) | Delta |
|---------|-----------------------|-------|
| `Vec.Push` | 0.42 | baseline |
| `Vec.PushInitialized` | 0.49 | +16.6% |

This win comes from eliding the temporary copy and letting LLVM attach tighter
alias scopes to the loads/stores generated inside the loop.

For code bases with heavy builder-style APIs, the new placement insertion and
removal helpers typically shave 10–20% off steady-state allocator pressure.

## Further Reading

* [Specification §4.4 Unsafe Contract](../../SPEC.md#unsafe-contract-maybeuninit)
* [`Std.Memory.MaybeUninit<T>` implementation](../../packages/std.core/src/memory.ch)
* [`Std.Collections.Vec` placement helpers](../../packages/std/src/collections/collections.ch)
