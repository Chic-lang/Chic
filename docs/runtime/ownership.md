# Runtime Ownership Hooks

Chic’s bootstrap runtime exposes a small set of drop-glue entry points so
host-managed containers can invoke the compiler-generated destructors for any
value that escapes into native code.

- `chic_rt_install_drop_table(entries: *const DropGlueEntry, len: usize)` installs the
  per-module drop table emitted by the LLVM backend. The installer runs during
  the module constructor phase and caches the `(type_id, fn ptr)` pairs in a
  lock-free registry.
- `chic_rt_drop_resolve(type_id: u64) -> Option<DropGlueFn>` resolves the thunk
  for a specific monomorphised type, falling back to the static table if nothing
  was registered dynamically.
- `chic_rt_drop_register(type_id: u64, fn ptr)` and
  `chic_rt_drop_clear()` are convenience hooks for host shims that need to
  override or flush entries at runtime (e.g., testing harnesses).

The standard library’s `Vec<T>` façade now calls `__drop_glue_of<T>()` when
creating vectors and forwards the resolved pointer to the runtime. When `T` is
trivially droppable the intrinsic returns `null` and the façade substitutes the
shared `__drop_noop` callback (surfaced to Chic code as
`Std.Runtime.DropRuntime.DropNoopPtr()`), keeping container fast paths
branch-free.
