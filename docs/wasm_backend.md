# Impact WebAssembly Backend

This document captures the design for the in-house WebAssembly backend that
replaces the retired Cranelift integration. The backend is implemented
as a *native plugin* inside Impact and is responsible for translating Chic
MIR into `.wasm` binaries that can be executed locally through Wasmtime or
embedded into other hosts.

The scope of this document:

* Define the value mapping between Chic types and WebAssembly value types.
* Describe the MIR → WebAssembly translation strategy (control-flow, locals,
  calls, async lowering hooks).
* Specify the calling convention and module surface used by Impact.
* Explain the runtime integration used by `chic build`, `chic run`, and
  `chic test`.
* Record open items and future extensions as GitHub issues so outstanding work
  stays discoverable and reviewable.

> **Note:** Native LLVM builds emit platform archives (`.a`/`.lib`) alongside
> Impact-native `.clrlib` packages that bundle objects, metadata, and manifests
> for linker-less reuse. The WebAssembly backend continues to generate `.wasm`
> (and optional `.wat`) artifacts exclusively.

### Current implementation status

* `chic build --backend wasm` lowers Chic functions with structured
  control flow (`if`, `while`/`for`, `switch`) into `.wasm` binaries. Literal
  `match` arms (including `_` wildcards) lower to chained comparisons; guards,
  bindings, irrefutable destructuring patterns, and aggregate projections
  across structs/enums/unions now lower without diagnostics. Async and generator
  terminators remain unsupported for now.
* `chic run --backend wasm` now executes `.wasm` artifacts through the
  in-house interpreter defined in `runtime/wasm_executor.rs`. The executor
  understands the opcode subset emitted by the backend (structured control
  flow, integer arithmetic, globals, linear-memory load/store operations, and
  the `chic_rt` host hooks) and returns the program exit code directly to the
  CLI.
* The backend reserves function imports from the `chic_rt` module for runtime
  coordination. In addition to deterministic termination hooks (`chic_rt.panic`
  → exit code 101, `chic_rt.abort` → exit code 134), the module now imports
  `chic_rt_throw(payload: i64, type_id: i64)` so unhandled exceptions can be
  forwarded to the host with their typed identity intact. Borrow/lifetime
  primitives (`borrow_shared`, `borrow_unique`, `borrow_release`) and resource
  management helpers (`drop_resource`, `await`, `yield`) remain in
  place. The in-tree executor implements these imports directly so Wasm builds
  can run without external tooling while still enforcing Chic’s borrow and
  error semantics.
  The `type_id` argument is the little-endian interpretation of the first eight
  bytes of the Blake3 digest of the error's canonical name, giving both the
  bootstrap runtime and the future unwinder a stable, comparable identity.
* `chic test --backend wasm` compiles the suite to Wasm and executes exported
  `test::…` functions through the in-house executor. Assertion traps or runtime
  faults (e.g., division by zero) surface as test failures; async testcases are
  still skipped until the async runtime lands.
* The `tests/codegen_exec.rs` harness runs `chic build` in a subprocess with
  explicit timeouts so integration tests fail fast instead of hanging on
  pathological builds.
* Wasmtime integration and richer runtime imports are tracked as GitHub issues.
  The current executor is a bootstrap implementation that keeps execution fully
  in-tree while the long-term host runtime is developed.

---

## 1. Value Representation

| Chic type                           | WebAssembly representation             |
|-----------------------------------------|----------------------------------------|
| `bool`                                  | `i32` (0 = false, non-zero = true)     |
| `int` (32-bit signed)                   | `i32`                                  |
| `uint` / `unsigned int`                 | `i32`                                  |
| `long` (64-bit signed)                  | `i64`                                  |
| `ulong` / `unsigned long`               | `i64`                                  |
| `float`                                 | `f32`                                  |
| `double`                                | `f64`                                  |
| Pointers/references (`in` / `ref` / `out`) | `i32` linear-memory address (host-managed) |
| Aggregates (struct/enum/tuple)          | Packed into linear memory; MIR exports
                                            address via stack slots              |
| Async state machines                    | Pinned frame stored in linear memory,
                                            pointer passed as `i32`              |

* All scalar locals are lowered to native Wasm locals. Complex values are
  spilled into a stack frame allocated in linear memory and referenced via
  `i32` addresses. This mirrors the MIR *Place* model and keeps borrow
  semantics intact for future alias analysis. The runtime executor now
  instantiates linear memory so these frame slots can be read and written
  faithfully during execution.
* Impact reserves memory page 0 as the mutable stack. Each function receives a
  shadow stack pointer local (`__chic_sp`) that is updated when values are
  materialised in memory. The executor initialises the mutable global backing
  this pointer and enforces bounds checks on every load/store.

### Zero-initialisation statements

`StatementKind::ZeroInit` and `ZeroInitRaw` lower to the bulk-memory
`memory.fill` instruction. Managed destinations resolve to an address +
length pair derived from the MIR place metadata; raw calls evaluate the
pointer/length operands and coerce them to `i32` (wrapping any `i64`
temporaries). When the intrinsic’s size is zero the backend elides the
instruction entirely. The resulting Wasm snippets mirror the LLVM lowering
(`ptr`, `i8 0`, `len`) without ever materialising raw pointers in Chic
source, and the regression suite exercises both forms via
`src/codegen/wasm/tests/function_emitter/memory.rs::zero_init_*`.

---

## 2. Function & Module Layout

1. Every Chic function lowers to a WebAssembly function.
   * MIR locals become Wasm locals. Non-scalar locals materialise in the linear
     stack via helper intrinsics.
   * Function signatures are derived from the MIR `FnSig`:
     * Parameters map to `i32`/`i64`/`f32`/`f64` per the table above.
     * Return values map to a single Wasm result; `Ty::Unit` becomes an empty
       result.
2. Executables export an entry point:
   * `Main` (root namespace) becomes an exported function named `chic_main`.
   * The module optionally exports `_start` when built with `--wasi`, calling
     `chic_main` and translating its return to a WASI exit status.
3. Static and dynamic libraries expose all `public` Chic functions as
   Wasm exports using their mangled names. Metadata (target triple, Impact
   version, build kind) is stored in a custom section `chic.metadata`.
4. Runtime intrinsics (allocator, async reactor stubs, testing hooks) are
   imported from a module named `chic_rt`. The Wasmtime host provides these
   symbols when running `chic run` or `chic test`.
5. Optimisation hints surface via a custom section `chic.hints` containing
   semicolon-separated entries `symbol:hot|always_inline` or
   `symbol:cold|never_inline`. Engines and profilers may consume this metadata;
   it is ignored by runtimes that do not recognise the section.

```
(module
  (type $t0 (func (param i32 i32) (result i32)))
  (import "chic_rt" "alloc" (func $chic_rt.alloc (param i32) (result i32)))
  (table ...)
  (memory (export "memory") 2) ;; 2 pages = 128 KiB initial stack
  (func $Impact.Example.Main (result i32)
    (local $sum i32)
    ;; MIR block lowering goes here
  )
  (export "chic_main" (func $Impact.Example.Main))
  (custom "chic.metadata" "target=wasm32-wasi;build=exe;version=0.1.0")
)
```

---

## 3. MIR → Wasm Lowering

### 3.1 Basic Blocks

* Each MIR `BasicBlock` is executed via a dispatcher loop that stores the next
  block index in a dedicated local and re-enters the loop with a `br` of depth
  1. This keeps the Wasm control stack well-structured while still supporting
  arbitrary Chic CFGs:
  1. All blocks are assigned indices and wrapped in individual `if` regions
     inside a single `loop`/`block` pair.
  2. The dispatcher compares the active block index against each candidate and
     executes the matching `if` body.
  3. `Terminator::Goto` and `SwitchInt` update the index and branch back to the
     loop (`br 1`), enabling loops and other back-edges without recursion.
  4. `Terminator::Return` exits both the loop and outer block (`br 2`), leaving
     the final result on the stack.
* This design keeps lowering simple while supporting arbitrary control flow,
  including loops and `switch` constructs produced by the MIR builder.

### 3.2 Statements

* `Assign` – emit Wasm ops for the Rvalue and store into the Place. Scalars use
  local `set`; aggregates write into linear memory via helper intrinsics. Field
  projections on structs, enums (via `Downcast`), and unions use `TypeLayout`
  offsets to address the correct byte ranges.
* `StorageLive` / `StorageDead` – translate to stack pointer adjustments so the
  interpreter and borrow checker maintain consistent lifetimes. These also
  enable future debug info. When a storage slot that holds an active borrow is
  killed, the backend emits a call to `chic_rt.borrow_release` so the runtime
  can update its refcounting state before the memory is reused.
* `Borrow` – emit `chic_rt.borrow_shared/borrow_unique` with the borrow ID and
  base pointer for the borrowed place. The backend tracks which locals store the
  returned borrow handle so subsequent assignments or drops release the borrow.
* `Deinit` / `Drop` – call the type's `dispose(ref this)` hook (when present),
  then release any outstanding borrows before the storage slot is reclaimed.
  `DeferDrop` markers are resolved earlier in the pipeline into these concrete
  statements so the backend only sees real cleanup primitives.
* `Assert` – lower to a compare + conditional trap (`unreachable`) with the
  diagnostic string stored in a data segment for future tooling.
* Pending constructs remain fenced off; the backend emits distinguishable
  traps so unimplemented lowering is obvious in tests.

### 3.3 Terminators

| Terminator          | Wasm lowering                                                            |
|---------------------|---------------------------------------------------------------------------|
| `Goto`              | Update block index local, `br` to dispatcher.                            |
| `SwitchInt`         | Compare against each integer arm; on match update the block index and `br 1` back to the dispatcher (future work may fold this into a `br_table`). |
| `Match`             | Evaluate the scrutinee once, compare against integral literal arms, and branch; `_` lowers to a direct jump, all others fall through to `otherwise`. Guarded arms reuse the MIR-generated guard blocks, which evaluate the condition and branch via `SwitchInt` before continuing into the selected arm. |
| `Return`            | Store return local, `br 2` to exit the dispatcher loop, then emit Wasm `return`. |
| `Call`              | Direct Wasm call to translated function or imported intrinsic.           |
| `Await`             | Push the future handle (currently lowered as `i32`) and call
                        `chic_rt.await`. The import returns the ready value, which is stored in
                        the destination local before branching to the compiled resume block.     |
| `Yield`             | Forward the yielded `i32` payload to `chic_rt.yield` and then branch to
                        the resume block. The runtime import currently echoes the payload so host
                        executors can observe it.                                                 |
| `Throw`             | Zero-extend the exception operand to `i64`, push the hashed error type ID,
                        call `chic_rt_throw`, then emit `unreachable`.                           |
| `Panic`             | Push the deterministic exit code (101), call `chic_rt.panic`, then `unreachable`. |
| `Unreachable`       | Emit Wasm `unreachable`.                                                  |

### 3.4 Async Integration

* The initial lowering treats async/generator state machines conservatively:
  * Futures and yielded values must currently fit in an `i32`. The executor
    simply echoes the handle/value back to the caller so tests can observe the
    control-flow without a full scheduler.
  * The MIR metadata (`AsyncStateMachine`) is recorded but not yet used to
    materialise a stack-frame layout; drop blocks remain no-ops until borrow
    semantics are wired up.
* Runtime stubs for `chic_rt.await` and `chic_rt.yield` live in
  `runtime::wasm_executor::executor`. They are minimal stubs used to keep the
  execution model deterministic while async/generator execution support is
  expanded.

### 3.5 Tensor fallback & bounds

* Tensor allocations materialise in linear memory using the existing stack bump
  path when the tensor fits inside the configured stack budget; larger shapes
  fall back to the heap import used by other bulk allocations. The chosen
  alignment is the maximum of the element width and the explicit `align`
  recorded in MIR; both the size and alignment are written into the runtime
  metadata stream so the executor can free deterministically.
* Views translate to pointer arithmetic over the base allocation:
  `base + offset + Σ(index_i * stride_i * elem_width)`. Stride vectors are
  validated against the shape before lowering; impossible layouts surface a
  diagnostic instead of emitting unsafe code.
* Copies prefer the fast path when both sides are contiguous and aligned
  (lowered to `memory.copy` with an `offset + len` guard). Strided copies use
  explicit nested loops that walk the shape in row-major order, computing byte
  offsets with the recorded strides.
* Bounds checks run once per tensor op: the backend computes the highest byte
  touched by the allocation/view/copy and compares it against the active stack
  pointer and linear-memory length. When the check fails the backend emits a
  deterministic `unreachable` so the executor reports a trap instead of
  silently wrapping addresses.
* Host shims for device transfers remain stubs: they record the attempted
  transfer (src/dst memspace, bytes, alignment) in the perf/diagnostic stream
  and then no-op so builds stay deterministic without an accelerator.
* Regression coverage lives in `tests/codegen/wasm/tensors.rs`, covering
  contiguous copies (`memory.copy`), strided loop fallbacks, view offset/stride
  handling, stack-vs-heap allocation planning, and bounds traps.

### 3.6 Quantized fallback

* Supported quantization policies in WASM: per-tensor and per-channel scales
  with `nearest_even` or `toward_zero` rounding and optional saturation. Other
  rounding modes are rejected with diagnostics at codegen time.
* Rounding is emitted as integer math in linear memory: multiply by the inverse
  scale, apply the zero-point, then truncate/round based on the policy. A clamp
  to the target bit width executes when saturation is requested.
* GEMM/conv fall back to nested loops that apply the same rounding/saturation
  sequence. No accelerator intrinsics are available in WASM, so the backend
  records the chosen policy in the perf stream instead.
* When an unsupported policy is encountered (e.g., stochastic rounding),
  lowering emits a deterministic diagnostic and a `unreachable` stub so the
  module still validates under the executor without silently changing semantics.
* Regression coverage: `tests/codegen/wasm/quantized.rs` mirrors the LLVM
  scaffolding so per-channel policies and rounding/saturation stay in sync
  across backends.

### 3.7 Accelerator stubs

* `src/codegen/wasm/accelerator.rs` records enqueue/copy/record/wait operations in order and emits
  diagnostics when unsupported accelerator features are requested. No GPU is required; the stub
  simply logs intent for profiling.
* Future work: propagate accelerator metadata into `mir.json` and perf traces once lowering hooks
  are wired; current stub only records ordering for tests.

---

## 4. Host Integration (`chic build/run/test`)

* `chic build --backend wasm` writes a `.wasm` artifact and a JSON cache entry
  (`.wasm.cache.json`) for incremental rebuilds. No native object file is
  produced.
* `chic run --backend wasm` executes the generated module through the
  bootstrap interpreter in `runtime::wasm_executor::{executor, module, parser}`,
  returning the program's exit code directly to the CLI.
* `chic test --backend wasm` compiles the suite to Wasm and executes exported
  `test::…` functions through the in-house executor. The executor has dedicated
  unit coverage for parser failures, control-stack underflow, global mutation,
  and arithmetic/memory traps so regressions are caught locally.
* `manifest.yaml` runtime settings (`runtime.wasm`) feed directly into `chic run/test`.
  invocations: memory limits are enforced before execution, the configured environment
  variables are applied, and feature flags are recorded in the run/test traces for tooling.
* WASI imports, runtime hooks, and Wasmtime integration remain planned work and
  are tracked as GitHub issues.

---

## 5. Follow-up

Follow-ups should be tracked as GitHub issues. When behavior changes, update this document and the relevant spec/docs references so the intended design stays discoverable.
