# Chic Runtime ABI

`runtime/include/chic_rt.h` is the canonical, checked-in header for the
bootstrap Chic runtime. The Rust implementation must behave as a thin
shim over this ABI; once the Chic-native runtime lands, the Rust path is
limited to symbol forwarding and diagnostics.

## Conventions

- All status returns are `i32` error codes; `StringError`/`VecError` are
  non-negative, `SharedError` is negative on failure. `Success` is always `0`.
- The ABI requires `__int128`/`unsigned __int128` support for string numeric
  formatting; compilation fails early if a target lacks 128-bit integers.
- Pointers passed to `*_set_*`, mutation helpers, and drop hooks must be
  non-null unless explicitly documented. Slice/handle structs are plain POD
  (`repr(C)`) and may be moved/copied freely by host code.
- All inline buffers are caller-owned; drop helpers must be invoked for any
  heap-backed representation before reusing the structs.

## Memory intrinsics

| Symbol | Signature | Notes |
|--------|-----------|-------|
| `chic_rt_alloc` | `*mut u8 chic_rt_alloc(usize size, usize align)` | Allocates `size` bytes with the requested alignment, returning `null` on failure. |
| `chic_rt_alloc_zeroed` | `*mut u8 chic_rt_alloc_zeroed(usize size, usize align)` | Same as `alloc` but clears the allocation before returning it. |
| `chic_rt_realloc` | `*mut u8 chic_rt_realloc(*mut u8 ptr, usize old_size, usize new_size, usize align)` | Resizes an allocation; passing `null` behaves like `alloc`. |
| `chic_rt_free` | `void chic_rt_free(*mut u8 ptr, usize size, usize align)` | Releases an allocation; `null` is a no-op. |
| `chic_rt_memcpy` | `void chic_rt_memcpy(*mut @restrict u8 dst, *const @restrict u8 src, usize len)` | Copy helper honouring the aliasing contract; debug builds assert that the ranges do not overlap. |
| `chic_rt_memmove` | `void chic_rt_memmove(*mut u8 dst, *const u8 src, usize len)` | Overlap-friendly copy. |
| `chic_rt_memset` | `void chic_rt_memset(*mut u8 dst, u8 value, usize len)` | Byte-wise fill helper used by string and span runtimes. |
| `chic_rt_zero_init` | `void chic_rt_zero_init(*mut u8 dst, usize len)` | New helper invoked whenever MIR `ZeroInit{,Raw}` statements cannot constant-fold to `memory.fill` / `llvm.memset`. Managed code never touches pointers directly; the compiler or runtime thunk routes the request here when lengths are dynamic. |
| `chic_rt_ptr_offset` | `*mut u8 chic_rt_ptr_offset(*mut u8 ptr, isize offset)` | Pointer arithmetic helper for low-level runtime glue. |

All functions use the system allocator today but are intentionally isolated so
future Chic-native runtimes can provide drop-in replacements. The memory
intrinsic table feeds backend docs (LLVM/Wasm lowering) and the `Std.Memory`
facade, keeping the ABI surface identical across bootstrap and Chic-native
implementations.

## Strings (`chic_rt_string_*`)

- Layout: `ChicString { ptr, len, cap, inline[32] }` with a tagged
  capacity bit; `ChicStr { ptr, len }` is an immutable slice.
- Inline capacity is fixed at 32 bytes; `cap` embeds an inline tag that must be
  preserved when mutating inline strings.
- Error codes: `StringError` maps to `i32` (Success, Utf8, CapacityOverflow,
  AllocationFailed, InvalidPointer, OutOfBounds).
- Mutators (`clone`, `clone_slice`, `reserve`, `push_slice`, `truncate`, append
  helpers) return status codes and treat null pointers as `InvalidPointer`.
  Append helpers expect optional alignment plus format specifiers (UTF-8
  slices); numeric formatting uses 128-bit intermediates.

## Vectors/Arrays (`chic_rt_vec_*`, `chic_rt_array_*`)

- Layout: `ChicVec` is `repr(C, align(16))` with fields `{ptr,len,cap,
  elem_size,elem_align,drop_fn,RegionHandle,uses_inline,inline[64]}`. Inline
  storage is active only when the element metadata fits; `uses_inline` toggles
  promotion/demotion.
- Companion types: `ChicVecView` and `ChicVecIter` are POD; drop
  callbacks use `extern "C" fn(uint8_t *)` and may be null.
- Error codes: `VecError` (Success, AllocationFailed, InvalidPointer,
  CapacityOverflow, OutOfBounds, LengthOverflow, IterationComplete).
- Constructors cover plain and region-backed vectors; mutation helpers
  (`push/pop/insert/remove/swap_remove/truncate/clear/set_len`) consume
  `Value{Const,Mut}Ptr` handles. Accessors expose data pointers, inline
  storage, element metadata, and iterator helpers; `vec_ptr_at` and `array_*`
  panic on invalid indices to preserve the existing ABI contract.

## Shared ownership (`chic_rt_{arc,rc,weak}_*`)

- Layout: handles are single-field pointer structs (`ChicArc`,
  `ChicWeak`, `ChicRc`, `ChicWeakRc`) targeting headers with
  refcounts, payload size/alignment, drop fn, and type id.
- Error codes: `SharedError` (Success=0, InvalidPointer=-1,
  AllocationFailed=-2, Overflow=-3). Drop functions and type ids are passed as
  `uintptr_t` to keep the C boundary agnostic.
- `*_new` copies payload bytes into managed allocations and registers resource
  tracking; clone/downgrade/upgrade adjust refcounts atomically (Arc) or via
  interior mutability (Rc). `*_get_mut` returns `null` unless the handle is
  uniquely owned. `chic_rt_object_new` allocates untyped storage using
  runtime type metadata.

## Test executor bridge

- The MIR interpreter test executor has no exported `extern "C"` surface yet.
  The header intentionally leaves this empty so Chic-native executors can
  add stable hooks (enumeration, async scheduling, panic reporting) without
  carrying earlier Rust semantics.

## Shim policy

The checked-in header is the ABI source of truth. Once the Chic runtime
is compiled natively, the Rust implementation is restricted to forwarding and
telemetry—no behavioural drift or duplicated logic is permitted.
