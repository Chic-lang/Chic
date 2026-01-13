# Local Functions – Usage & Debugging

Local functions let you structure complex methods without allocating heap closures. This note
captures a few practical tips beyond the formal specification (see
[`SPEC.md`](../../SPEC.md#local-functions-section324)).

## Mental model

* **Non‑capturing locals** lower to plain functions with `internal` linkage. The generated symbol
  name is `Parent::local$N::Name`, where `Parent` is the qualified name of the enclosing function and
  `N` is the zero-based declaration index inside that body.
* **Capturing locals** behave exactly like lambdas plus a name. Every capture is stored in an
  environment struct named `Parent::local_env#N`. The lowering pass automatically instantiates the
  environment whenever the local function value is used, so writes to captured variables after the
  instantiation are visible to subsequent calls.
* **Generics and constraints** from the parent scope remain in scope, and you can declare extra
  generic parameters on the local function itself. The type checker enforces `where` clauses and
  effect annotations the same way that it does for namespace members.

## Debugging checklist

1. **Find the symbol name.** When stepping through LLVM IR or native assembly, search for
   `::local$`. Each increment corresponds to the declaration order inside the parent function.
2. **Inspect the capture layout.** The environment struct is emitted as `Parent::local_env#N`. Each
   captured field keeps the original variable name, which makes it easy to correlate debugger views
   with source code.
3. **Hidden parameters.** Capturing locals always receive the environment pointer as their first
   argument. If you set breakpoints on the thunk, the debugger will show the hidden pointer in
   addition to the user-defined parameters.
4. **Borrow checker diagnostics.** Moves into a local function consume the original binding. If you
   see a "value borrowed here after move" diagnostic, confirm that the capture mode matches the
   desired semantics (e.g., wrap values in `rc`/`arc` when sharing across multiple locals).

## Tips

* Prefer local functions when you need multiple entry points inside a method and the surrounding
  state is large. They avoid heap allocation and keep environments deterministic.
* When interoperating with APIs that expect `fn` pointers, use `.to_fn_ptr()` on the local function
  value. Non-capturing locals convert implicitly; capturing locals require the explicit call so the
  compiler can enforce lifetime rules.
* Keep names short and meaningful—the mangled `local$N` slot is for tooling only, so source readers
  should still understand what the local function does.
