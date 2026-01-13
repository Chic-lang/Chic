# Pedantic Clippy Guardrail

`cargo xtask lint-pedantic` runs Clippy with `-W clippy::pedantic` (capped at warnings) and fails if any diagnostics appear in:

- `src/codegen/isa.rs`
- `src/syntax/expr/precedence.rs`
- `src/syntax/expr/builders.rs`
- `src/syntax/expr/parser/calls.rs`
- `src/syntax/expr/parser/operators.rs`
- `src/syntax/expr/parser/lambda.rs`
- `src/syntax/expr/parser/inline_asm.rs`
- `src/syntax/expr/parser/primary.rs`

The scope is intentionally narrow while we burn down existing pedantic noise; expand the guard patterns as modules are cleaned up. The parser guard list covers the expression precedence table, AST builders, and the expression parser shards (calls/operators/lambda/inline_asm/primary), all pedantic-clean with targeted allowances where structural symmetry matters. Use this command before landing changes that touch the guarded areas so regressions are caught early without being blocked by unrelated warnings elsewhere in the workspace.
