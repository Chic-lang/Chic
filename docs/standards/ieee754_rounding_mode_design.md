# IEEE 754-2019 Rounding-Mode Strategy

Context: Design note following the inventory in `docs/standards/ieee754_inventory.md`.

## Goals

- Allow Chic programs to select IEEE 754-2019 rounding modes and have that choice respected across parsing/const-eval, MIR, LLVM/WASM backends, runtime helpers, and the in-tree WASM executor.
- Preserve cross-backend parity (LLVM + WASM) and enable status-flag collection (invalid, div-by-zero, overflow, underflow, inexact) in the same execution context that owns the rounding mode.
- Keep rounding semantics explicit in MIR (per-op metadata with a well-defined default) and avoid backend-specific ad hoc switches.

## Rounding Model

- **Default**: `NearestTiesToEven` (IEEE “roundTiesToEven”), matching current host behaviour. This is the process-wide default for parsing and const-eval.
- **Explicit modes**: `TowardZero`, `TowardPositive`, `TowardNegative`, `TowardNearestTiesToAway` (full IEEE set).
- **Granularity**:
  - Global *float environment* (`FloatEnv`) stored per thread/executor with fields `{ rounding_mode, status_flags }`.
  - Per-operation override on MIR FP ops and FP↔int casts (`Rvalue::Binary`, `Rvalue::Unary` for `Neg` on FP, `CastKind::{IntToFloat,FloatToInt,FloatToFloat}`), plus intrinsics (e.g., `DecimalIntrinsic` already carries a rounding operand).
  - If an op has `rounding: None`, the backend uses `FloatEnv.rounding_mode`.
- **Const-eval**: honour the same metadata; const-folded FP ops must be executed with the requested rounding mode (software path if host cannot supply it).

## MIR Representation

- Add `enum RoundingMode { NearestTiesToEven, NearestTiesToAway, TowardZero, TowardPositive, TowardNegative }`.
- Extend MIR FP-bearing instructions with `rounding: Option<RoundingMode>`:
  - `Rvalue::Binary` when both operands are FP.
  - `Rvalue::Unary` when operand is FP (covers `Neg`).
  - `Rvalue::Cast` for `FloatToInt`, `IntToFloat`, `FloatToFloat`.
- Introduce `FloatEnv` in MIR interpreter/runtime metadata to hold `{ rounding_mode, status_flags }`, threaded through codegen entrypoints.
- Keep `NumericLiteralMetadata` untouched but allow const-eval to consult `FloatEnv.rounding_mode`.

## Backend Mapping

- **LLVM**:
  - Use constrained intrinsics (`llvm.experimental.constrained.*`) for FP arithmetic and casts when `rounding` is set or when status flags are enabled; attach `rounding` metadata string (`"rne"`, `"rtz"`, `"rtp"`, `"rtn"`, `"rna"`).
  - Default-mode ops without flag collection may stay on plain `fadd`/`fsub`/`fmul`/`fdiv`/`frem` to preserve current performance; if flag collection is enabled, prefer constrained intrinsics everywhere for uniformity.
  - Emit `llvm.experimental.constrained.set.rounding` when the global `FloatEnv` changes, scoped to the function via prologue/epilogue helper or per-block if the env is mutated.
  - Capture exceptions via `fpexcept` operand on constrained intrinsics, forwarding bits into the runtime flag sink.
- **WASM**:
  - Native instructions only support `NearestTiesToEven`; use them when `rounding` is `None`/`NearestTiesToEven`.
  - For other rounding modes, lower FP ops to calls into runtime helpers (`chic_float_add_f32`, `..._f64`, etc.) that implement IEEE rounding in software and update status flags in `FloatEnv`.
  - FP casts map similarly: native `trunc`/`convert` for default mode; runtime shims for alternative modes.
  - The in-tree WASM executor mirrors this: execute native ops for default rounding, dispatch to the same helper functions for non-default rounding and accumulate flags in executor state.

## Runtime / CLI Surface

- Provide thread-local `FloatEnv` accessor in the runtime (`get_rounding_mode`, `set_rounding_mode`, `clear_flags`, `read_flags`) used by both native and WASM executor paths.
- CLI/testing hooks: `CHIC_ROUNDING_MODE={rne,rna,rtz,rtp,rtn}` to seed the default env for driver/tests; tracing flag to log rounding-mode transitions.

## Testing / Acceptance

- Golden tests for each rounding mode covering add/sub/mul/div/rem/neg and FP↔int conversions for edge cases (subnormals, infinities, ±0, NaN payloads).
- Cross-backend parity suite (LLVM IR execution + WASM executor) using the shared helpers to assert identical bit patterns and status-flag snapshots.
- Const-eval tests that fold FP expressions under non-default rounding and match runtime execution.

## Open Questions / Follow-ups

- Evaluate using constrained intrinsics for *all* FP ops (even default mode) to simplify status-flag collection; measure perf impact before locking in.
- Decide whether to permit per-expression rounding-mode annotations in source syntax or expose only via runtime API initially.
