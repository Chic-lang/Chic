# Chic Allocator Hooks

Chic exposes a pluggable allocator surface so hosts can override the
default global allocator without expanding the Rust shim. The hooks are
portable across LLVM/WASM and apply to every runtime allocation path
(`GlobalAllocator`, `Vec`, `String`, regions, etc.).

## VTable layout

- Location: `packages/std.alloc/src/mod.ch`
- Struct: `Std.Alloc.AllocatorVTable { Context, Alloc, AllocZeroed, Realloc, Free }`
  - Function pointer fields are `isize` values; set to `0` to fall back to the
    default runtime allocator.
  - `Context` is forwarded to every hook so implementations can thread state.
- Runtime ABI: `runtime/include/chic_rt.h::ChicAllocatorVTable`.

### Typed pointer ABI

- `chic_rt_alloc*` and `Std.Memory.{Alloc,AllocZeroed,Realloc}` return
  `ValueMutPtr { ptr, size, align }` handles.  The runtime populates `size` and
  `align` with the requested layout even when allocation fails so downstream
  helpers (`MaybeUninit`, span/vec construction, stackalloc) can propagate
  typed metadata without re-computing it.
- `Std.Memory.StackAlloc.Buffer<T>` exposes stack-backed `Value{Const,Mut}Ptr`
  handles (byte-sized/aligned) so IO bridges and other callers can validate
  typed spans without reintroducing raw pointer overloads.
- LLVM and WASM share the same `Value{Const,Mut}Ptr` layout; backend mappers
  use the recorded struct layout rather than ad-hoc pointer-sized tuples.
- Copy/move/set helpers on `Std.Memory.GlobalAllocator` accept the same typed
  handles, so IO/stackalloc/collections no longer traffic in raw pointers.

## Installing a custom allocator

```chic
import Std.Alloc;

// Assume these static externs are defined in Chic with `@extern("C")`.
extern isize my_alloc;
extern isize my_alloc_zeroed;
extern isize my_realloc;
extern isize my_free;

let table = AllocatorVTable.With(
    context: Std.Numeric.UIntPtr.Zero.AsPointer<byte>(),
    alloc: my_alloc,
    allocZeroed: my_alloc_zeroed,
    realloc: my_realloc,
    free: my_free
);
Hooks.Install(table);
```

Use `Hooks.Reset()` to restore the default allocator, and
`Hooks.Telemetry()`/`Hooks.ResetTelemetry()` to inspect/reset allocation
counters.

## Telemetry

`AllocationTelemetry` mirrors the runtime counters:

- `AllocCalls`/`AllocZeroedCalls`
- `AllocBytes`/`AllocZeroedBytes`
- `ReallocCalls`/`ReallocBytes`
- `FreeCalls`/`FreedBytes`

Telemetry is recorded regardless of whether a custom allocator is installed.

## Selection & layering

- **Hosted targets (`std`):** Prefer the platform default (leave vtable entries
  as zero) so hooks ride the OS allocator. Install a custom allocator only when
  instrumentation or isolation is required.
- **Freestanding (`#![no_std]`):** Install an arena/bump allocator early in the
  boot flow and guard heap-backed APIs behind feature flags. Builds link
  `alloc`/`foundation` only when `CHIC_ENABLE_ALLOC=1`; keep the flag unset to
  remain heapless.
- **Deterministic services:** Pair a region allocator with strict telemetry
  asserts in tests. Reset telemetry before/after high-traffic sections to catch
  leaks or unexpected growth.
- **Diagnostics:** Layer selection guidance lives in `docs/guides/stdlib_layers.md`.

## Notes

- The default allocator uses the Rust global allocator. Custom allocators are
  expected to honour alignment and to tolerate zero-sized allocations.
- Hooks apply to both host and WASM backends; WASM builds receive the same
  vtable and counters.
- `#![no_std]` crates link the allocator surface (and the `foundation` crate)
  only when `CHIC_ENABLE_ALLOC=1` is set at build time; keep the flag unset to
  leave the heap disconnected on freestanding targets.
