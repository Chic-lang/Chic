# Borrow + Cleanup Audit (2025-02-21)

## Spec vs implementation snapshot
- **Spec (Chic 0.10):** moves invalidate sources, borrows are call-scoped (CL0031 on escape), and deterministic destruction runs exactly once for every owned value on every exit path (fallthrough, return, throw, break/continue/goto, async/generator drop), in reverse lexical order.
- **Implementation (today):** scopes track locals but `pop_scope` never emits `StorageDead`, so plain locals are left live when control leaves a block. `lower_return_statement` and `emit_throw` do not shed scopes; `drop_lowering` only lowers active `DeferDrop` lists on `Return/Throw/Panic` and relies on upstream `StorageDead` to drop everything else. `using`/`region` lowerings only emit `StorageDead` when the current block has no terminator; `emit_throw` inside a `try` rewrites to `Goto` (not a drop-triggering terminator), so deferred drops are skipped. Result: owned locals commonly leak and drop order is undefined.

## Bug list (current vs expected)

- **Cleanup correctness — function exit drops elided**
  - *Expected:* Function-scoped owned locals drop before any `return` (explicit or implicit) and on fallthrough. Reverse lexical order.
  - *Current:* `pop_scope` is a no-op and `lower_return_statement` sets a terminator without dropping. Blocks with no terminator are later patched to `Return` without any `StorageDead`. MIR lacks drops for locals that require destruction.
  - *Repro:*  
    ```chic
    struct Droppy { dispose(ref this) { Std.Debug.log("drop"); } }
    public fn Leak() { var d = Droppy(); return; }
    ```  
    MIR ends with `Return` and no `StorageDead`/`Drop` for `d`; `dispose` never runs.
  - *Fix plan:* Emit scope drops when scopes close and before `Return`/implicit return; ensure root scope is drained.
  - *Tests:* MIR/compile-run fixtures covering implicit/explicit return drop order.

- **Cleanup correctness — deferred drops lost on non-throw unwinds (try/using)**
  - *Expected:* `using`/`lock`/`fixed`/`region` resources drop on any exit, including `throw` paths that dispatch to catch/finally via gotos.
  - *Current:* `emit_throw` in a `try` sets `Goto` to handlers; `drop_lowering` does not treat `Goto` as a drop point, so active `DeferDrop` entries are skipped. Resources survive the throw path.
  - *Repro:*  
    ```chic
    struct Droppy { dispose(ref this) { Std.Debug.log("drop"); } }
    public fn UsingThrow() {
        try {
            using var r = Droppy();
            throw new Exception();
        } catch(Exception _) {}
    }
    ```  
    The `r` drop is absent on the exceptional edge.
  - *Fix plan:* Record try-entry scope depth and drop to it before rewiring throws to handlers; make deferred drops run for handler gotos.
  - *Tests:* Compile-run and MIR fixtures asserting drop execution on throw into catch/finally.

- **Cleanup correctness — deferred drops lost on `goto`/label exits**
  - *Expected:* Resources created in a `using`/`region` scope drop even when control jumps via `goto` out of the scope.
  - *Current:* When the body installs a `Goto` terminator, `lower_using_statement` skips `StorageDead`, and `drop_lowering` ignores active drops because `Goto` is not in `terminator_requires_drop`. The resource is never dropped.
  - *Repro:*  
    ```chic
    struct Droppy { dispose(ref this) { Std.Debug.log("drop"); } }
    public fn UsingGoto(bool flag) {
        using var r = Droppy();
        if flag { goto after; }
        after: ;
    }
    ```  
    On the `goto` path, no drop is emitted for `r`.
  - *Fix plan:* Either emit `StorageDead` before installing `Goto` or make drop lowering aware of scope exit edges. Prefer emitting drops at the lowering site so active drop stacks are flushed.
  - *Tests:* Compile-fail/diagnostic MIR fixture ensuring drops appear before gotos that leave the scope.

## Test and fix plan (summary)
- Introduce a unified scope-drop insertion:
  - Drop live locals when a scope ends (including block fallthrough).
  - Drop all current scopes before `Return` and before try-dispatch throws; plumb try scope depth.
  - Ensure `using`/`region` resources emit drops even with `Goto`/handler dispatch terminators.
- Add regression tests:
  - MIR builder tests for drop insertion on explicit/implicit returns and goto/throw edges.
  - Runtime tests that record `dispose` order across fallthrough/return/throw/goto.
  - Negative tests remain enforced for borrow escape (CL0031) and use-after-move; expand if new borrow bugs are found during fixes.
