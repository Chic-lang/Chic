## Concurrency Memory Model Baseline

This note records the invariants shared by the language surface, MIR, backends,
runtime, and developer tooling so changes land consistently across the stack.

### Guiding Principles

- **C11-style semantics** – Chic adopts the familiar five memory-order
  lattice (`Relaxed`, `Acquire`, `Release`, `AcqRel`, `SeqCst`) and pairs it with
  fence expressions. Atomic operations must obey the "failure ordering is never
  stronger than success ordering" rule to guarantee portability across CPU
  backends.
- **Streams/events are linear** – `Stream<M>`/`Event<M>` carry `MemSpace`
  generics. Enqueues require unique stream access, and the borrow checker ties
  captured buffers to their completion events until a matching `wait(event)`
  executes. Streams captured across `await` must be `@pinned` (or use a pinned
  memspace) so DMA handles remain stable.
- **Auto-trait enforcement** – Every value moved across thread or async splice
  points requires `ThreadSafe`; shared references additionally require
  `Shareable`. Diagnostics in the MM-series build on top of the existing
  TCK/TCK async enforcement but surface concurrency-specific guidance.
- **Chic-first runtime** – Mutexes, readers-writer locks, condition
  variables, `Once`, and atomic primitives remain part of the Chic
  standard library surface while delegating blocking/parker logic to the
  runtime. Any temporary bootstrap shim must preserve behaviour across native
  and WASM targets.

### Layer Responsibilities

| Layer          | Responsibilities | Guard Rails |
|----------------|------------------------------|-------------|
| Language & Parser | Surface `Std::Sync::MemoryOrder`, syntactic sugar for `Atomic<T>` members, and `atomic {}` blocks that desugar into MIR intrinsics. | Reject invalid modifiers early and emit parser diagnostics keyed to MM-series codes. |
| Type Checker & MIR builders | Thread auto-trait constraints through async lowering, produce new MIR rvalues/statements for atomic load/store/RMW/fence, and ensure const-eval rejects non-const intrinsics. | No MIR is considered valid unless verification knows how to walk the atomic forms. |
| Backends (LLVM/WASM) | Lower MIR atomics to backend primitives, preserving ordering and alignment, with WASM falling back gracefully when threads are unavailable. | LLVM and WASM must stay feature-matched before closing any atomic subtask. |
| Runtime & Stdlib | Provide Chic implementations of atomic primitives and synchronisation helpers while exposing async-safe adapters (e.g. `AwaitableMutex`). | Runtime-backed handles (native & WASM) must park instead of spinning, preserve documented APIs, and version their hooks so ABI drift remains visible to tooling. |
| Tooling | Extend lint passes and IDE hints to understand MM-series diagnostics and new APIs. | Keep diagnostics and docs in sync with the spec. |

### Acceptance Gates

- Changes that touch the concurrency model update the relevant spec/tutorial sections and add regression coverage in `tests/`.
- Litmus tests (LB/SB/IRIW and friends) must pass on both LLVM and WASM targets
  before marking the testing subtask finished.

### Litmus Catalogue

The canonical Chic litmus suite lives under `tests/concurrency/litmus`. Each scenario is a
`testcase` that spawns real `Std.Platform.Thread` workloads, synchronises them with a shared `StartGate`,
and records the observed outcomes via `Std.Sync::Atomic*` instances:

- `store_buffering.ch` – two writers publish then observe, asserting `(0, 0)` never occurs under
  Acquire/Release ordering.
- `load_buffering.ch` – two readers load then publish, forbidding `(1, 1)` when both loads are
  synchronised via release stores.
- `iriw.ch` – interleaved readers of two writers, ensuring we never observe inconsistent `(1,0)`
  vs `(0,1)` pairs.
- `message_passing.ch` – classic flag/data release/acquire message passing confirming the payload
  becomes visible as soon as the flag trips.

`tests/concurrency.rs` drives the suite through `tests/concurrency/litmus/mod.rs`, compiling the
Chic sources with `CompilerDriver::run_tests` for both LLVM and WASM backends. CI treats any
non-`Passed` testcase status as a failure and reports the offending case names in the panic message.

### Cross References

- Spec source of truth: `SPEC.md` §7 "Concurrency"
- Thread-safety background: `docs/compiler/thread_safety.md`

### Effect Families (AI-First Additions)

Chic’s effect system extends beyond exception typing. The runtime, borrow
checker, and concurrency model must enforce the following capabilities:

| Effect    | Purpose | Key Rules |
|-----------|---------|-----------|
| `random`  | Deterministic RNG usage, seeded via explicit `RNG` handles. | All random draws must pass an `RNG` value; async tasks capture RNG state by value; tooling records provenance in `mir.json`. |
| `measure` | Probabilistic log-prob accumulation (`sample`/`observe`). | Must appear with or after `random`; MIR emits `Observe` ops and accumulators; inference engines consume the recorded trace. |
| `network` | Distributed collectives and actor messaging. | Functions that send/receive over the network declare the effect; capability tokens ensure only authorised code performs I/O. |
| `cancel`  | Structured concurrency cancellation. | Scopes that initiate cancellation (`cancel_scope`, `.cancel_rest()`) must declare the effect; callee responsibilities are explicit in diagnostics. |

These effects are tracked in MIR metadata and surfaced through `mir.json`/`hints.json`. Sections
§16.4–§16.10 of the language spec contain the normative rules.
