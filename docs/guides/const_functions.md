# Const Functions & CTFE

Chic `const fn` bodies execute entirely at compile time. They are folded into MIR so constants, statics, and default arguments embed their computed values without any runtime work.

## Supported Surface

- Declare const-friendly functions with `const fn Name(params) -> Return { ... }`.
- Valid statements: blocks, `const`/`var` with initialisers, `if`/`else`, expression statements, and `return`.
- Valid expressions: literals, identifiers, unary/binary ops, casts, calls to other `const fn`/CTFE intrinsics, assignments to local identifiers, member access on constant structs/enums, `sizeof`/`alignof`/`nameof`, and `quote(...)`.
- Disallowed signatures ([TCK160]): `async`, `extern`, `unsafe`, `throws`, generic parameters, and `ref`/`out` bindings.
- Disallowed bodies ([TCK161]): loops, `try`/`using`/`lock`, `goto`/`yield`, object construction/indexing, lambdas, `await`, and other runtime-only constructs.

## Tooling & Diagnostics

- [TCK160] triggers when a const fn signature is not CTFE-safe (async/extern/unsafe/throws/ref/out/generic).
- [TCK161] fires when the body uses an unsupported statement or expression.
- CTFE metrics (`ConstEvalMetrics`) expose `fn_cache_hits`/`fn_cache_misses`, fuel consumption, and memoisation hit rates; enable `target=const_eval` logging to inspect them during builds.
- Bench: `cargo bench const_eval` exercises cached const fn evaluation to track regressions.

## Best Practices for Library Authors

1. Keep const fn signatures pure: prefer pass-by-value inputs, avoid effects, and return plain data.
2. Avoid loops and dynamic allocation—prefer recursion-free arithmetic/table generation or pre-baked lookup data.
3. Keep expressions simple so folding succeeds under the fuel limit; large computations should live in build scripts instead.
4. Add targeted tests that call `ConstEvalContext::evaluate_expression` to lock in folding behaviour and spot regressions.
5. When a construct is rejected with [TCK161], refactor toward supported building blocks (e.g., replace `while` with guarded `if` chains or precomputed tables).
