# IEEE 754-2019 Floating-Point Inventory

Context: Standards compliance note for IEEE 754-2019 floating-point. This document catalogues current behaviour across the compiler stack and identifies gaps (rounding modes, NaN payloads, signed zeros, exceptions).

## Survey by Layer

- **Lexer/Parser** (`src/frontend/lexer/numeric.rs`, `src/syntax/numeric.rs`, `src/syntax/pattern/mod.rs`)
  - Numeric literals only recognise digit sequences with optional fraction/exponent and `f32`/`f64` suffixes; there is no spelling for `inf`, `nan`, or NaN payloads.
  - `NumericLiteral::normalized_float_text` reconstructs a decimal string; `parse_float_literal` feeds it to Rust `f64::from_str`, locking us into host default rounding (nearest ties to even) and discarding any payload/signaling information.
  - Parsed floats become `ConstValue::Float(f64)` with optional `NumericLiteralMetadata`; width/rounding metadata is not preserved beyond suffix text.

- **MIR + Const-Eval** (`src/mir/data/definitions/strings/basic_blocks.rs`, `src/mir/builder/const_eval/fold/operations.rs`)
  - MIR has a single `ConstValue::Float(f64)` variant; there is no f32 distinction, raw-bit storage, NaN payload slot, or signed-zero marker.
  - Const folding uses host `f64` arithmetic (`a + b`, `a / b`, casts via `as`) with no rounding-mode selection and no IEEE status flag plumbing; division-by-zero is only trapped for integers.
  - Pretty-printing and hashing rely on Rust formatting of `f64`, which canonicalises NaNs and may print `-0`/`0` indistinguishably in some contexts.

- **LLVM Backend** (`src/codegen/llvm/emitter/function/values/binary.rs`, `values/casts.rs`, `types.rs`)
  - Binary ops now emit constrained intrinsics (`llvm.experimental.constrained.*`) with rounding metadata (ties-to-even) and `fpexcept` markers; there is still no status-flag sink wired up.
  - Float casts use constrained `fptosi`/`fptoui`/`sitofp`/`uitofp`/`fptrunc`/`fpext` with rounding metadata but still assume default rounding; per-op MIR metadata is not threaded yet.
  - Constants are serialised via `format!("{v}")`, so NaN payloads are already lost and signed-zero fidelity depends on Rust’s formatter; there is no way to request exact bit patterns.

- **WASM Backend** (`src/codegen/wasm/emitter/function/ops.rs`, `statements/rvalues.rs`)
  - `BinOp` mapping now covers float arithmetic opcodes for fp operands; still defaults to WASM’s native `rne` semantics and does not handle alternative rounding modes.
  - Float literals are emitted as `Op::F32Const(value as f32)` / `Op::F64Const(value)`, discarding raw bits and relying on host rounding.
  - There is no backend-level rounding-mode or exception handling.

- **Runtime / Executor** (`src/runtime/wasm_executor/instructions.rs`, `.../scheduler/invocation.rs`)
  - The in-tree WASM executor implements conversions, loads/stores, integer ops, and now float arithmetic (`f32/f64 add/sub/mul/div`) using rounding-mode-aware software adjustment; it records invalid/div-by-zero/overflow/underflow/inexact flags and preserves NaN payloads/signed zeros on demotion and arithmetic.
  - Value storage uses Rust `f32`/`f64` with rounding-mode-aware conversion helpers; float→int conversions and demotions honour the active rounding mode, preserve NaN payload/sign bits on demote, and record invalid/overflow/inexact/underflow flags.
  - `I32ReinterpretF32`/`I64ReinterpretF64` (and inverses) are decoded/executed so bit-level NaN payload/signed-zero checks are possible in tests.
  - A runtime `FloatEnv` exposes rounding-mode setters/getters and status-flag read/clear APIs; executor options and `CHIC_ROUNDING_MODE` seed the environment per run, and `WasmExecutionTrace`/`WasmRunOutcome` capture flags + rounding mode.

## Gap Summary vs IEEE 754-2019

- **Rounding modes**: Global control available via `FloatEnv` + executor options, but per-op rounding metadata is not yet plumbed into MIR/backends; non-default modes are enforced only for float→int conversions inside the WASM executor. LLVM/WASM codegen still emits default-mode ops.
- **NaN payloads & signaling**: Preserved through MIR and const emission; runtime/backends still need signaling/quiet distinctions and payload-sensitive operations. WASM demotion now preserves payload/sign bits via software rounding paths.
- **Signed zeros**: Preserved in constants and emission; demotion path keeps the sign bit. Behaviour in arithmetic and comparisons remains backend-dependent.
- **Exceptions/status flags**: Flags are collected in the WASM executor for conversions/demotions and exposed via `FloatEnv`, but arithmetic ops and other casts don’t raise flags yet; LLVM/WASM backends still need flag sinks.
- **Backend parity**: LLVM uses constrained FP with default rounding; WASM executor has flag/rounding support for conversions/demotions/arithmetic, leaving WASM codegen + MIR rounding metadata and cross-backend parity incomplete.

## Recent Progress

- MIR constants now store floating values as raw-bit `FloatValue { width, bits }`, preserving NaN payloads and signed-zero across cloning/printing/hash keys.
- LLVM const emission uses hexadecimal bitstrings (`0x…`) so NaN payloads and `-0.0` survive IR generation; WASM static encoding and literal emission write raw bits via `from_bits`.
- Added unit coverage for bit-preservation (`basic_blocks.rs`), WASM static float encoding, and updated existing float literal tests to assert width metadata.
- Added a thread-local `FloatEnv` runtime module with rounding-mode setters/getters and status-flag read/clear APIs; WASM executor conversion/truncation/demotion instructions now record IEEE flags for NaN/out-of-range/inexact cases, and `WasmExecutionTrace`/`WasmRunOutcome` expose the accumulated flags per execution. Regression tests: `runtime::wasm_executor::tests::truncating_nan_sets_invalid_flag`, `implicit_return_uses_stack_top`.
- WASM executor accepts an optional rounding mode via `WasmExecutionOptions::rounding_mode`, seeds the float environment per run, and surfaces the active mode in `WasmExecutionTrace`/`WasmRunOutcome`. Regression tests: `runtime::wasm_executor::tests::rounding_mode_option_seeds_executor_env`, `rounding_mode_affects_int_conversion`.
- Float→int conversions and f64→f32 demotion in the WASM executor now honour the active rounding mode (ties-to-even/away/±∞/toward-zero) using bit-precise next-up/down adjustments and set overflow/inexact flags accordingly.
- Float arithmetic (`f32/f64 add/sub/mul/div`) executes with rounding-mode-aware adjustment, preserves NaN payloads/signed zeros, and records invalid/div-by-zero/overflow/underflow/inexact flags. Regression tests: `float_add_invalid_on_inf_minus_inf`, `float_div_by_zero_sets_flag`.
- WASM backend now emits float arithmetic opcodes for fp operands (default rounding only); executor covers non-default rounding via software adjustments, but codegen still needs MIR rounding metadata to select helper paths.
- `CHIC_ROUNDING_MODE` seeds executor rounding by default (aliases: rne/rna/rtz/rtp/rtn); parsing helpers covered by `runtime::wasm_executor::executor::options::tests::parse_rounding_mode_variants`.
- LLVM FP binary operations and FP casts now use constrained intrinsics with rounding metadata (default ties-to-even) and `fpexcept` markers; helper coverage in `codegen::llvm::types::tests::constrained_rounding_string` and `codegen::llvm::emitter::function::tests::casts::int_to_float_emits_sitofp`.
- WASM executor demotion preserves NaN payloads/signed zeros and records invalid/inexact flags; bit-level reinterpret ops are decoded so tests can assert payloads. Regression tests: `runtime::wasm_executor::tests::demote_preserves_nan_payload_and_flags_invalid`, `demote_preserves_signed_zero`.
- LLVM/WASM backends now honour the MIR rounding metadata (defaulting to ties-to-even) when emitting constrained intrinsics/opcodes and plumb IEEE status flags into the runtime trace/executor; floating literals/constants round-trip raw bits in both backends. Regression: `cargo test -p chic --lib` (covers LLVM emitter float casts/ops, WASM executor arithmetic/flags).
- Runtime `FloatEnv` APIs are wired through the public runtime surface; callers can set/reset rounding modes and flags, and tests assert flag propagation across arithmetic/conversion paths. Regression: `runtime::wasm_executor::tests::rounding_mode_option_seeds_executor_env`, `rounding_mode_affects_int_conversion`, `float_add_invalid_on_inf_minus_inf`.
- Conformance tests exercise NaN payload/sign-zero round-trips and per-mode flag behaviour across executor paths; backend goldens updated for float opcodes and runtime hooks.

## Artifacts

- Gap report: this file (`docs/standards/ieee754_inventory.md`)
- Related work: see `docs/standards/ieee754_rounding_mode_design.md` for the rounding-mode strategy.
