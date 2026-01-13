# `Vec<T>` ↔ `Array<T>` Conversions

This note captures the runtime and ABI semantics for converting between Chic `Vec<T>` instances and fixed-length `Array<T>` buffers.  Both data structures use the same runtime storage descriptor (`ChicVec` in `chic_rt::vec`) consisting of:

```
ptr        // pointer to element buffer (may be null for zero-sized types)
len        // number of live elements
cap        // reserved element capacity
elem_size  // size in bytes of each element
elem_align // element alignment
drop_fn    // optional drop glue callback
```

Readonly arrays reuse this descriptor with additional type metadata in the compiler.

## Goals

* Provide well-defined, zero-copy ownership transfer between vectors and arrays when the underlying buffer satisfies fixed-length requirements.
* Offer clone-based conversions when ownership must be preserved (e.g., `Vec<T>.ToArray()`).
* Preserve element drop glue: conversions must neither leak resources nor drop elements twice.
* Maintain consistent error reporting via `VecError`.

## Runtime Entry Points

The runtime exposes four helpers, all returning a `VecError` status (`0 = Success`):

| Function | Purpose |
|----------|---------|
| `chic_rt_vec_into_array(dest, src)` | Moves the buffer from `Vec<T>` (`src`) into `Array<T>` (`dest`) and empties `src`. Reuses the buffer when `cap == len` (or `T` is zero-sized); otherwise allocates a new buffer sized to `len`, memcpy’s the elements, and frees the old allocation without running element drop glue. |
| `chic_rt_vec_copy_to_array(dest, src)` | Clones a `Vec<T>` into an `Array<T>`. The destination’s capacity is shrunk to match its length on success. |
| `chic_rt_array_into_vec(dest, src)` | Moves the buffer from `Array<T>` to `Vec<T>`, emptying the source. No additional allocation is required. |
| `chic_rt_array_copy_to_vec(dest, src)` | Clones an `Array<T>` into a `Vec<T>` using the existing `vec_clone` logic. |

All functions validate their pointers and return `VecError::InvalidPointer` when either argument is null. Allocation failures surface as `VecError::AllocationFailed`. After a successful move (`*_into_*`), the source container is reset to its empty state (`len = 0`, `cap = 0` for non-zero-sized types, `usize::MAX` for zero-sized types).

## Ownership vs Copy Semantics

* `Vec<T>.IntoArray()` (standard-library façade) wraps `chic_rt_vec_into_array`, transferring ownership and zero-copying whenever the vector is already tightly packed. When extra capacity exists, the runtime clones into a new buffer and frees the original allocation.
* `Vec<T>.ToArray()` calls `chic_rt_vec_copy_to_array` and leaves the original vector untouched.
* `Array<T>.IntoVec()` mirrors the move behaviour in the opposite direction.
* `Array<T>.ToVec()` uses `chic_rt_array_copy_to_vec` to clone the buffer while retaining the array.

The helpers preserve element drop glue by moving the `drop_fn` callback with the buffer and only invoking it when the destination is later dropped.

## Error Handling

* Invalid pointers → `VecError::InvalidPointer`
* Allocation failure during copy → `VecError::AllocationFailed`
* Other error codes currently propagate from existing utilities (`VecError::CapacityOverflow` when size computations overflow).

## Codegen Expectations

LLVM and WASM backends import the new runtime symbols directly (`chic_rt_vec_into_array`, `chic_rt_vec_copy_to_array`, `chic_rt_array_into_vec`, `chic_rt_array_copy_to_vec`). No new MIR operations are required—the standard-library façade invokes the helpers via normal function calls.

## Standard Library Surface

`Std.Collections.Vec` and `Std.Collections.Array` expose ergonomic wrappers:

```
public static VecError Vec.IntoArray(ref VecPtr vec, out ArrayPtr array);
public static VecError Vec.ToArray(in VecPtr vec, out ArrayPtr array);
public static VecError Array.IntoVec(ref ArrayPtr array, out VecPtr vec);
public static VecError Array.ToVec(in ArrayPtr array, out VecPtr vec);
```

These return the runtime status so callers can surface diagnostics.  Standard-library diagnostics will evolve alongside higher-level collection APIs.
