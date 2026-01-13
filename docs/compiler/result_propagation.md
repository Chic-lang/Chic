# Result Propagation (`?`) – Implementation Notes

The bootstrapper now lowers Chic’s postfix `?` operator end-to-end, covering parsing, MIR generation, async integration, and diagnostics.

## Operator semantics

- The parser recognises postfix `?` via `ExprNode::TryPropagate`, preserving the operand and span for downstream passes (`src/syntax/expr/parser/calls.rs`, `src/syntax/expr/parser/tests.rs`).
- MIR lowering rewrites `operand?` into a `match` over the result enum, splitting into `Ok`/`Err` blocks:
  - The `Ok` branch moves (and coerces) the payload into a fresh temporary before resuming the enclosing control flow.
  - The `Err` branch constructs the enclosing function’s result enum and emits an early `Return`, guaranteeing that all temporaries created for the operand are retired (`emit_storage_dead`) prior to the exit (`src/mir/builder/body_builder/expressions/control.rs`).
  - When the operand and return error payloads differ, the lowering searches for `TargetError::from(SourceError)` via the symbol index and inserts the call, mirroring Rust’s `From` requirement. Missing conversions raise a focused diagnostic.
- Expression statements (`result?;`) are now lowered directly (the statement dispatcher delegates to `lower_try_propagate_expr`), ensuring the control-flow graph is identical irrespective of whether the propagated value is assigned or discarded (`src/mir/builder/body_builder/expressions/mod.rs`).
- Async functions treat the propagation match like any other terminator: the `Err` arm returns before the state machine records a suspension point, while the `Ok` arm flows into the subsequent `Await`. A new regression test exercises this path (`src/mir/builder/tests/async_support.rs`).

## Diagnostics

- Unknown operand type: ``?` requires an enum `Result<T, E>` operand; `Foo` is not an enum`.
- Mismatched enclosing return type: ``?` requires the enclosing function to return `Result<_, _>`; found `Bar``.
- Missing error conversion: `cannot convert error type `Demo::ErrorA` to `Demo::ErrorB`; implement `Demo::ErrorB::from(Demo::ErrorA)``.
- The `?` operator is implemented end-to-end; MIR lowering succeeds for supported operands.

All diagnostics originate from the lowering pass so type checking continues to operate on existing constraint kinds.

## Regression coverage

| Area                          | Tests                                                                                                                                              |
|------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------|
| Control-flow lowering        | `mir::builder::tests::control_flow::{lowers_result_propagation_into_match,result_propagation_in_trait_impl_lowering,result_propagation_uses_from_conversion_when_available}` |
| Conversion diagnostics       | `mir::builder::tests::control_flow::result_propagation_reports_missing_conversion`                                                                 |
| Type / operand validation    | `mir::builder::tests::control_flow::{result_propagation_requires_result_types,result_propagation_requires_result_operand}`                         |
| Async integration            | `mir::builder::tests::async_support::async_result_propagation_returns_before_await_on_error`                                                       |
| Parser + AST preservation    | `src/syntax/expr/parser/tests.rs::parses_postfix_result_propagation`, `src/frontend/parser/tests/grammar/statements.rs::parses_result_propagation` |

## Follow-up work

- Extend borrow-check specific regressions to ensure propagated temporaries obey existing drop-order invariants (the builders already emit `StorageDead`, but explicit checks would guard regressions).
- Add backend-level coverage once native codegen is enabled so the LLVM/WASM stacks are validated against the new control flow.

Related spec updates live in `SPEC.md`.
