# LLVM call terminators

This module now splits the call terminator lowering across focused helpers so each path is testable in isolation:

- `calls.rs` orchestrates dispatch and hands off to submodules.
- `calls/args.rs` marshals operands to typed LLVM arg strings with consistent validation.
- `calls/dispatch.rs` covers trait-object, virtual, and function-pointer calls (including vtable slot resolution and bitcasts).
- `calls/runtime.rs` handles object-new, decimal/startup runtime shims, and shared IO/string helpers.

When adding a new path:
1. Extend the orchestrator to delegate to a helper instead of inlining logic.
2. Prefer `render_args_for_signature` / `render_args_for_types` so argument count/type errors surface with context.
3. Add an IR regression test under `src/codegen/llvm/emitter/function/tests/calls.rs` that exercises the path end-to-end (direct, trait/virtual, indirect, runtime/async).
4. Keep cleanup deterministic: every call terminator must end with an explicit branch to the target block and store through `store_place` only after type checks.

Reference IR snapshots:
- Async startup call emission (`chic_rt_startup_call_entry_async`) marshals five pointer-width arguments and branches to `%bb1`.
- Function-pointer calls bitcast the loaded `i8*` to the typed signature before invocation and branch to the target block.

Coverage: run `cargo test --lib codegen::llvm::emitter::function::tests::calls` followed by a targeted `cargo llvm-cov --lib --json --output-path coverage/terminator_calls_local.json -- codegen::llvm::emitter::function::tests::calls::`.
