# Scope Cleanup Invariants

- **Exactly once:** Every owned local drops exactly once. The lowering pipeline emits `StorageDead` on every scope exit edge; drop lowering expands those markers into `Deinit`/`Drop` in reverse lexical order.
- **All exit shapes covered:** Fallthrough at the end of a block, `return`, `throw` (including try-dispatch rewrites), `break`/`continue`, `goto`/labels, and async/generator drop paths all shed the scopes they exit before jumping.
- **Order:** Drops run inside-out (innermost scope first) and within a scope in reverse binding order, with views dropped before their owners. `StorageDead` always trails the corresponding drops.
- **Partial init:** When a scope exits before all fields/locals are initialised, only the live bindings are dropped; uninitialised slots stay untouched. `out`/uninitialised locals must not be read and only drop after an assignment makes them live.
- **Mechanics:** `pop_scope` inserts `StorageDead` for any still-live locals on fallthrough, `drop_to_scope_depth` drains scopes before control-flow edges that leave them, and `emit_throw`/`lower_return_statement` call it so deferred drops are not lost when terminators rewrite to `Goto`. `goto` lowering snapshots scope locals and injects `StorageDead` when jumping to shallower depths.
