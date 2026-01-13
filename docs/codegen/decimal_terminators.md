# Decimal Terminator Flow

The LLVM backend now treats every decimal terminator as the same three-stage
pipeline regardless of whether the MIR originated from `Rvalue::DecimalIntrinsic`
or from a call to `Std::Decimal::Intrinsics::*`.

```
Decimal call / intrinsic
        |
        v
collect_decimal_parts + encode rounding (shared::collect_decimal_parts)
        |
        v
vectorize hint constant?
   /             \
  yes             no
  |               |
  v               v
single runtime    emit_vectorized_runtime_result
symbol (scalar    ├─ call scalar runtime
or simd)          ├─ call simd runtime
  |               ├─ select status/value/variant on runtime flag
  v               └─ assemble DecimalIntrinsicResult
assemble DecimalIntrinsicResult
```

## Module layout

- `helpers.rs` keeps type lookups, rounding encodings, and result assembly helpers shared by the intrinsic/runtime call paths.
- `intrinsic.rs` lowers MIR decimal intrinsics (including constant vectorise hints); `wrappers.rs` handles the public `Std.Decimal.Intrinsics::*` surface.
- `runtime.rs` owns runtime symbol selection and call marshalling; `shared.rs` stays minimal (typed values, vectorise selection, runtime symbol helpers) to keep tests focused.

## Regression tests

- IR fixtures in `src/codegen/llvm/emitter/function/tests/decimal.rs` now cover scalar vs SIMD wrappers (`Add`, `AddVectorized*`, `Fma`, `FmaVectorized`) plus options/rounding paths and explicit argument-count failures.
- Unit tests in `terminators/decimal/tests.rs` exercise wrapper spec detection and op mappings.
- Coverage for the decimal terminator modules sits above 85% (`helpers` ~87%, `intrinsic` ~85%, `wrappers` ~92%) when running `cargo test --lib codegen::llvm::emitter::function::tests::decimal`.

* **Operand prep** – `collect_decimal_parts` translates each operand into the
  `{ i32, i32, i32, i32 }` struct consumed by the runtime, so add/sub/mul
  terminators no longer duplicate the conversion loops.
* **Constant hints** – `vectorize_hint_constant` detects enum literals so that
  existing fast paths (e.g. `AddVectorized`) can directly target the SIMD
  runtime without branching.
* **Dynamic hints** – `emit_vectorized_runtime_result` issues both scalar and
  SIMD runtime calls, stitches their `(status, value)` pairs together with
  `select i1` instructions, and reports the chosen variant. This helper is
  invoked both from MIR decimal intrinsics and from the `*WithOptions`
  wrappers, so coverage stays tied to the shared logic.

## Extending the lowering

1. **Add new runtime symbols** – extend `decimal_runtime_symbol` with the scalar
   and SIMD names. The shared helpers already expect the pair; unit tests in
   `decimal/shared.rs` assert that add/sub/mul entries exist.
2. **Hook new wrappers** – wire the symbol into `wrapper_spec` so the call
   terminator routes into `emit_decimal_wrapper_call`. The wrapper will either
   use `emit_decimal_intrinsic_fixed` (constant flags) or
   `emit_decimal_intrinsic_with_options` (dynamic vectorize) which now funnels
   through the helpers.
3. **Author tests** – add an IR regression test under
   `src/codegen/llvm/emitter/function/tests/decimal.rs` that calls the wrapper
   and asserts the emitted runtime calls/`select` plumbing. Use the existing
   `decimal_with_options_fn` builder as a template.

With this structure, adding `Std::Decimal::Intrinsics::DivWithOptions` or
additional rounding-aware wrappers only requires updating one helper rather than
spreading the logic across duplicated loops.
