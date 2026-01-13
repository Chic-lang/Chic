# MIR Strings Module Layout

The `src/mir/data/definitions/strings` directory now separates interning, module assembly, and metadata plumbing into dedicated files while re-exporting a stable public facade.

## Module Map

- `strings/mod.rs` - Thin façade (≈16 LOC) re-exporting the specialised submodules and keeping the public surface stable.
- `strings/interning.rs` - Owns `StrId`, `StrLifetime`, and `InternedStr`, keeping the interning data model isolated from module metadata.
- `strings/module.rs` - Provides `MirModule`, focusing on module assembly and delegating metadata fields to `module_metadata`.
- `strings/module_metadata.rs` - Hosts `Export`, `GlobalAllocator`, and `ModuleAttributes` plus defaults, giving lowering/serialization code a clear surface for metadata concerns.
- `strings/types.rs` - Encapsulates the core `Ty` family plus container helpers (`ArrayTy`, `TupleTy`, `FnTy`, etc.).
- `strings/functions.rs` - Houses `MirFunction`, `FunctionKind`, and `FnSig` definitions referenced throughout the pipeline.
- `strings/basic_blocks.rs` - Contains `MirBody`, local/block/statement/terminator structures, and pattern/operand representations.
- `strings/utils.rs` - Shared helper utilities (type conversion/canonical names, MIR constructors) consumed by the above modules.

## Key Consumers

- `mir::builder::module_lowering::{driver,pipeline,attributes}` import metadata via `mir::module_metadata`.
- `codegen::metadata::{mod,functions}` consume exported symbols and allocator state from the new module.
- Test fixtures in `codegen::wasm` and `runtime::test_executor` construct `ModuleAttributes` through the re-exported module metadata facade.

## Metrics & Guards

- LOC split after the latest refactor: `mod.rs` 16, `types.rs` 306, `functions.rs` 43, `basic_blocks.rs` 628, `utils.rs` 220, `interning.rs` 32, `module.rs` 27, `module_metadata.rs` 38.
- Regression coverage: `cargo test mir::data::definitions::strings::tests` (unit tests) and `cargo check` for the broader crate.
- Formatting is enforced with `cargo fmt`.

Future additions to the strings data model should extend the relevant submodule rather than growing `mod.rs`, keeping each file under the 300-400 LOC guideline.

## Contributor Notes

- Use `strings::utils::ty_from_type_expr` and `canonical_ty_name` for AST-to-MIR conversions instead of reimplementing ad-hoc helpers.
- Construct new MIR bodies or places via `utils::new_mir_body` / `utils::new_place` to keep initialisation consistent.
- When extending strings data structures, add the new type or helper to the appropriate module (`types`, `functions`, `basic_blocks`, or `utils`) to maintain the split and lint budgets noted above.

## Function Pointer Layout

- `Ty::Fn` lowers to an intrinsic struct layout emitted by `TypeLayoutTable::ensure_fn_layout`, keeping backend handling consistent for closures and overload adapters.
- Field order: `invoke` (ptr to thunk), `context` (ptr to capture env or null), `drop_glue` (ptr to env drop thunk), `type_id` (`u64` from `drop_type_identity`), `env_size` (`usize`), `env_align` (`usize`).
- `.to_fn_ptr()` and overload coercions populate every slot; drop glue thunks call `chic_rt_drop_invoke` + `chic_rt_closure_env_free` to run capture destructors and free heap environments.
- Backends must project the `invoke` + `context` fields for indirect calls and trap on null `invoke` values to mirror runtime safety checks.

## AI-First Op Families

Spec §16 introduces several new MIR statement/terminator families. This section tracks the canonical layout for each so backend teams and tooling remain aligned.

### Tensor Operations

- **Value kinds:** `TensorOwn` (owns storage) and `TensorView` (borrowed slice).
- **Statements:**
  - `TensorAlloc { place, element_ty, shape, layout, memspace, align }`
  - `TensorView { place, base, view_shape, view_stride, offset }`
  - `TensorCopy { dst, src }` (explicit copy; zero hidden allocations)
  - `TensorFree { place }` (emitted when an owning tensor leaves scope)
- **Metadata:** Each tensor local records `ShapeMetadata { dims, symbolic_bounds }`, `LayoutMetadata { trait_id, stride }`, and `MemSpaceMetadata { trait_id }`. This metadata is serialised into `mir.json` so agents can inspect layout/stride information.
- **Borrow checking:** Views retain pointers to the base owner; drop scheduling ensures the base outlives all active views.

### Differentiable Programming

- **Statements:**
  - `AdjointAlloc { place, primal }`
  - `AdjointAccumulate { target, value }`
  - `Checkpoint { id, live_locals }`
  - `Remat { block_id }` (forces recomputation instead of storing)
- **Metadata:** Gradient bodies are emitted as `$grad` functions with `GradientMetadata { mode, primal_symbol, tape_size, checkpoints }`. Verification ensures SSA pairing between primal and adjoint values.

### Deterministic Randomness & Probabilistic Effects

- **Statements:** `SplitRng`, `AdvanceRng`, `RandScalar`, `RandTensor`, `Sample`, `Observe`, `AddLogProb`.
- **Metadata:** Every random site records `(rng_local, lexical_id, span)`; probabilistic ops track distribution symbol names and target effect sets. Tooling uses this to thread seeds and reconstruct log-prob traces.

### Accelerator & Stream Model

- **Statements/Terminators:**
  - `EnqueueKernel { stream, kernel, args, launch_dims }`
  - `EnqueueCopy { stream, dst, src, bytes, kind }`
  - `RecordEvent { stream, event }`
  - `WaitEvent { event }`
  - `Barrier { scope }`
- **Metadata:** Streams and events carry `device_id`, `memspace`, and `ordering`. Events remain live until every waiting block completes; metadata is exported so schedule planners can overlap copies/compute safely.
- **Body fields:** `MirBody::stream_metadata` records `(stream_id, local, mem_space)` pairs. The new `AcceleratorBuilder` assigns deterministic `stream_id`s when emitting enqueue/record/wait statements so `mir.json` can correlate queues across functions/backends.
- **Borrowing:** Enqueue and record statements synthesise shared borrows for the stream and any operand places, releasing them when a matching `WaitEvent` executes. Moving buffers or streams before the corresponding wait triggers borrow errors, preventing use-after-free while DMA is in flight.

### Graph & Schedule Lowering

- **Statements:** Graph lowering introduces `GraphBegin`, `GraphEnd`, and `GraphNode { id, op_kind, inputs, outputs }`. Schedule directives annotate the surrounding graph metadata rather than emitting extra statements.
- **Profiles:** `ScheduleProfileMetadata { graph, profile_hash, directives }` is attached to the function body, enabling tooling to verify deterministic profile usage.

### Structured Concurrency & Cancellation

- **Statements/Terminators:**
  - `ScopeBegin/ScopeEnd` for `cancel_scope`
  - `Spawn { callee, args }`
  - `AwaitAny { handles }`
  - `InstallTimeout { scope_id, deadline }`
  - `CancelToken { handle }`
- **Metadata:** Each scope records a `cancel_token_id` so tooling can ensure spawned tasks are awaited or explicitly cancelled. Effects metadata (`cancel`) is exported alongside the MIR function.
