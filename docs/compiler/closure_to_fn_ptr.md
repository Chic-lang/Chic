# Closure `.to_fn_ptr()` Notes

_Updated: 2025-11-03_

## Updated lowering pipeline

- Chic closures still materialise as struct "environments" with a synthetic
  `lambda#N::Invoke` thunk that receives the captured values followed by the call-site
  arguments.
- `.to_fn_ptr()` now builds a uniform `{ invoke, context, drop_glue, env_size, env_align }`
  record (layout name `fn(...) -> ...`). The `invoke` slot points to a compiler-synthesised
  adapter whose signature is `(context: *mut u8, params…)`; codegen always prepends the
  stored `context` before user arguments when issuing the indirect call.
- Capturing closures copy the environment to the heap via `chic_rt_closure_env_clone`,
  record the appropriate drop glue, and set `env_size`/`env_align` so the generated fn
  drop glue can both call the destructor and free the allocation.
- Non-capturing functions still coerce, but now via a tiny adapter that ignores the
  context argument and forwards directly to the target symbol. All fn pointers share
  the same call ABI (context + params).

## Runtime/codegen behaviour

- LLVM/WASM backends treat `Ty::Fn` as an aggregate in memory; indirect calls load
  `invoke`/`context` fields and prepend the context argument before dispatch. Signature
  tables include the extra `ptr` parameter so tables remain type-safe.
- Drop glue for fn values is generated alongside other types. It invokes the recorded
  environment drop glue (if any) via `chic_rt_drop_invoke` and frees the heap
  allocation with `chic_rt_closure_env_free`.

## Diagnostics and coverage

- Capturing closures still require an explicit `.to_fn_ptr()`; the builder error remains
  but now points at the adapter-based lowering. `Ty::Fn` participates in drop analysis so
  escaped callbacks free their environments.
