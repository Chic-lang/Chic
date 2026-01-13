# Chic Core Library

The Chic core library lives under `src/core` and is bundled into both
standard and `#![no_std]` builds. It provides the minimal surface for value/result
containers, slice primitives, and ownership markers without pulling in platform
wrappers.

Core is the base of the layering stack: add `alloc` + `foundation` to opt into
heap-backed collections, and add `std` only when platform wrappers (IO, sockets,
threads, async) are required.

- **Location:** `src/core`
- **Namespace:** Types continue to live under the `Std` namespace so existing
  imports remain valid (`import Std;` brings `Option`, `Result`, and `Span`
  into scope).
- **Runtime policy:** Core types rely only on runtime pointer/type metadata and
  the zero-initialisation intrinsics; they never reach into platform-specific
  std wrappers.

## Option / Result

- `Option<T>` stores a `_hasValue` flag plus the payload. `None()` zeroes the
  payload via `Std.Core.CoreIntrinsics.InitializeDefault`, so `out` parameters
  always leave the caller with a well-defined value.
- `Result<T, E>` mirrors the same layout with `_isOk`, `_ok`, and `_err` fields.
- `Expect` throws `Std.InvalidOperationException` when unwrapping a missing
  value so callers can handle the failure using normal exception control flow.

```chic
import Std;

let maybe = Option<int>.Some(5);
var value = 0;
if (maybe.IsSome(out value) && value == 5) { /* ... */ }

let parse = Result<int, string>.FromOk(42);
var parsed = 0;
if (!parse.IsOk(out parsed)) { throw new Std.FormatException("parse failed"); }
```

## Spans

- `Span<T>`/`ReadOnlySpan<T>` wrap the runtime `SpanPtr`/`ReadOnlySpanPtr`
  handles. Length/stride metadata must describe a valid, element-aligned
  contiguous region; all slicing helpers route through the runtime guards.
- Guarding: invalid ranges, stride mismatches, or out-of-bounds slices throw
  `Std.IndexOutOfRangeException`/`Std.ArgumentException` with the corresponding
  `SpanError` code recorded in the diagnostic path.
- Zero-length spans are always represented with a null data pointer and zeroed
  metadata so they can be passed across FFI safely.

```chic
var buffer = Span<int>.StackAlloc(4);
buffer.CopyFrom(ReadOnlySpan<int>.FromArray([1, 2, 3, 4]));
let tail = buffer.Slice(2);
let ro = tail.AsReadOnly();
```

## Copy / Drop Traits

- `Std.Copy` is a marker trait implemented for primitives (`bool`, integers,
  floats), pointer-sized integers, and `string`. It signals trivially-copyable
  values to the borrow checker and codegen layers.
- `Std.Drop` is a marker for deterministic destruction; types continue to use
  `dispose(ref this)` for custom destructors, and drop glue generation honours
  the marker when emitting runtime entries.

## Layering

- `core` is always present; add `alloc` + `foundation` for heap-backed, platform-free collections or `std` for platform wrappers.
- See `docs/guides/stdlib_layers.md` and `docs/runtime/allocators.md` for allocator configuration details.
