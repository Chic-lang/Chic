# Drop Glue Intrinsic Plan

Last updated: 2024-XX-XX

## Motivation

Standard-library containers (`Vec<T>`, future `HashMap`, etc.) and runtime
helpers need a uniform way to destroy element values that honours Chic’s
drop semantics (field order, custom `dispose`, nested drops) across both native
and WASM backends. The runtime ABI expects C-style function pointers, but the
current compiler only emits Chic-ABI `dispose(ref this)` methods.

## Design Summary

- During monomorphisation, emit a per-type drop thunk with C ABI:
  `__cl_drop::<T>(void*)`.
- Each thunk bitcasts the pointer to `T*` and reuses MIR’s existing drop
  lowering to destroy the value (respecting lexical drop order).
- Cache thunks per `T` to avoid duplicate emission.
- Expose an intrinsic `__drop_glue_of<T>() -> (fn @extern("C")(void*) -> void)?`
  that returns:
  - `null` when `needs_drop(T)` is false (callers can substitute `__drop_noop`), or
  - The address of `__cl_drop::<T>` when a drop is required.
- Provide a shared `@extern("C") void __drop_noop(void*)` in the standard library
  for the trivial case and surface its pointer via
  `Std.Runtime.DropRuntime.DropNoopPtr()`.

## Codegen Notes

- LLVM: emit the thunk as a normal function with `extern "C"` calling convention.
- WASM: generate a function with signature `(param i32)` and ensure the thunk is
  reachable via the function table if `call_indirect` is required.
- Monomorphisation must embed references to the thunk in MIR constants so both
  backends can materialise the pointer value.

## Runtime Integration

- Update `chic_rt_vec_*` and similar helpers to accept
  `(fn @extern("C")(void*) -> void)` pointers.
- Callers (e.g., `Vec<T>.dispose`, runtime reserve/reallocation paths) pass the
  pointer returned by `__drop_glue_of<T>()` or fall back to
  `Std.Runtime.DropRuntime.DropNoopPtr()` (which resolves to `__drop_noop`).
- When a container removes a single element (`Pop`, `RemoveAt`), invoke the glue
  directly before shifting elements.

## Diagnostics / Tooling

- Emit diagnostics when glue cannot be generated (e.g., recursive types without
  definition, missing `unsafe` prerequisites).
- Expose metadata so tooling can distinguish compiler-generated glue from
  user-authored `dispose`.
- Document nullable function-pointer syntax: `(fn @extern("C")(void*) -> void)?`.

## Test Plan

- Unit tests covering:
  - Simple aggregates (struct with fields requiring drop).
  - Nested drops (e.g., `Vec<Vec<String>>`).
  - Union/enum drop paths.
- Integration tests exercising:
  - `Vec<T>` on both LLVM and WASM backends (ensure no leaks and deterministic
    drop ordering).
  - Interaction with `__drop_noop` for trivially droppable types.
- Future regression tests for containers using the intrinsic (HashMap/HashSet).
