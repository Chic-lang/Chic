# Closure ↔ Function Pointer Design Notes

This note captures the intended representation of first-class function pointers and closure adapters in Chic.  The items below highlight the missing infrastructure that blocks implementation today.

## Target ABI

* `ImpactFnPtr` should be a `#[repr(C)]` struct containing `{ invoke: extern "C" fn(*mut u8, ...), context: *mut u8 }`.
* Non-capturing closures lower to `invoke` with a null context.
* Capturing closures materialise an adapter thunk that loads the captured environment via `context` and forwards to the closure body.  The thunk must share drop glue logic with existing closure structs.

## Compiler Responsibilities

1. **Monomorphisation** – generate adapter functions for every closure whose address escapes via `.to_fn_ptr()`.  The adapter owns the environment and calls borrow-check verified drop glue when the pointer is dropped.
2. **Borrow Checker** – ensure `.to_fn_ptr()` is permitted only when the captured environment outlives the function pointer usage (similar to move-out checks).
3. **MIR/Typeck** – treat `fn` values as `{invoke, context}` pairs.  Calls must load both words and invoke the runtime ABI.
4. **Codegen** – LLVM/WASM emitters need to bitcast the pair to the target representation and perform indirect calls.  Wasm must model the pair as two locals due to the lack of native multi-value pointers.

## Missing Primitives / Blockers

* Unsafe pointer types (`*mut T`) are not yet supported (see tasks 1.22).  The ABI requires passing raw pointers for the closure environment.
* No drop-glue intrinsic exists for closures; `.to_fn_ptr()` needs `__drop_glue_of<T>()` to surface the environment destructor.
* Run-time support for allocating and dropping closure environments outside the stack is absent.  Executors need shared logic to keep the environment alive across async and thread boundaries.

Until those pieces land, the compiler cannot safely emit the adapter representation described here.
