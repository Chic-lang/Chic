# Decimal Numeric Type Pipeline

Chic treats `decimal` as a first-class primitive so that lexical analysis, type checking, MIR construction, and both backends agree on size, layout, and semantics. This note aggregates the implementation details that keep the pipeline aligned.

## Primitive representation

- `TypeChecker::new` seeds `builtin_types` with `decimal`, ensuring resolver/typeck treat the symbol as a primitive alias alongside `System::Decimal` (`src/typeck/arena.rs:569`).
- The MIR layout table synthesises entries for `decimal`, `System::Decimal`, and `Std::Numeric::Decimal` so every layer receives the canonical 128‑bit, 16‑byte-aligned record (`src/mir/layout/builtins.rs:110-187`).
- `type_metadata::synthesise_type_metadata` records `decimal` in the builtin set so the runtime metadata tables expose the size/alignment pair even if the stdlib does not refer to the type directly (`src/type_metadata.rs:155-208`).

## Literals, parsing, and constant evaluation

- The lexer accepts both `m`/`M` suffixes for decimal literals and records the parsed integral/fractional components in `NumericLiteral` (`src/frontend/lexer/state.rs:744-779`).
- `syntax::numeric` exposes `NumericLiteralType::Decimal` metadata that propagates explicit suffixes into the AST and type checker (`src/syntax/numeric.rs:46-155`).
- Literal parsing delegates to `Decimal128`, which enforces the Chic scale limits and provides conversion helpers for constant folding (`src/decimal.rs`).
- The expression parser converts decimal tokens into `ConstValue::Decimal` nodes so constant evaluation can fold arithmetic, comparisons, and conversions (`src/syntax/expr/parser/primary.rs:649-657`, `src/mir/builder/const_eval/fold/operations.rs:31-771`).

## MIR lowering and intrinsics

- MIR bodies emit `Rvalue::DecimalIntrinsic` nodes for the Std.Numeric.Decimal surface helpers, capturing the intrinsic kind, rounding operand, and SIMD hint (`src/mir/data/definitions/strings/basic_blocks.rs:517-583`).
- The lowering tests cover scalar/vector intrinsic entry points (`src/mir/builder/tests/calls.rs`, `src/mir/builder/tests/decimal_intrinsics.rs`) and constant propagation of decimal expressions (`src/mir/builder/tests/consts.rs`).
- Bodies that request `@vectorize(decimal)` carry a `vectorize_decimal` flag so codegen can select scalar vs SIMD thunks (`src/mir/builder/functions.rs:238-405`).
- Direct callers of `Std.Numeric.Decimal.Intrinsics` must opt into `@vectorize(decimal)`; otherwise DM0002 surfaces and callers should migrate to the `Std.Numeric.Decimal.Fast` façade for scalar-only usage.

## Runtime and ABI

- `Decimal128` exposes conversions to bit arrays/u128 so both runtime adapters can marshal arguments across FFI boundaries without Rust-specific helpers (`src/decimal.rs:46-211`).
- `runtime::decimal` defines the C ABI surface (`DecimalRuntimeResult`, `DecimalConstPtr`, `DecimalMutPtr`, rounding encodings) and implements scalar + SIMD entry points that honour the banker’s rounding mode (`src/runtime/decimal.rs`).
- Stdlib wrappers in `Std.Numeric.Decimal` describe the Chic-facing structs/enums and runtime intrinsics, providing safe helpers for sums, dot products, and matrix multiplies while threading the vectorisation hint and diagnostics (`packages/std/src/numeric/decimal_types.ch`, `decimal_intrinsics.ch`, `decimal_fast.ch`).

## Backends

- LLVM maps `decimal` to `i128` and wires every intrinsic symbol to a runtime declaration, enabling direct emission of runtime calls during function lowering (`src/codegen/llvm/types.rs:33-45`, `src/codegen/llvm/signatures.rs:93-312`).
- The WASM backend exposes the same runtime hooks and marshalling buffers; `runtime_hooks.rs` adds typed outputs for scalar and SIMD forms so the host adapter can inspect status codes (`src/codegen/wasm/runtime_hooks.rs:101-135`).

## Regression suite

- Lexer coverage verifies lowercase/uppercase suffixes and diagnostics for malformed literals (`src/frontend/lexer/state.rs:744-811`).
- `src/decimal.rs` unit tests cover parsing, precision limits, arithmetic helpers, and error mapping.
- MIR builder tests cover constants, intrinsic lowering, attribute enforcement, and constexpr folding (`src/mir/builder/tests/{consts.rs,calls.rs,decimal_intrinsics.rs}`).
- End-to-end runtime glue is exercised via `tests/decimal_intrinsics.rs` once array/span literals land; meanwhile, benches in `benches/decimal_fast.rs` stress the scalar/SIMD helpers for performance regressions.

See `SPEC.md#decimal-type` for the language-level definition of precision, rounding modes, and literal syntax.
