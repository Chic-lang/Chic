# Async Runtime: Executor & ABI

*Updated: 2025-11-27*

Chic ships a native async runtime (Std executor) and an in-tree WASM executor that
share the same ABI. Async bodies still produce `AsyncStateMachine` metadata inside MIR; LLVM/WASM
codegen wires `Await`/`Yield` terminators to runtime hooks so compiled binaries execute outside the
MIR interpreter.

## Runtime ABI (native + WASM)

`src/runtime/async_runtime.rs` exposes a stable ABI mirrored by the WASM executor bridge:

- `FutureHeader` (`state_pointer`, `vtable_pointer`, `executor_context`, `flags`) and
  `FutureVTable` (`poll`, `drop`) sit at the front of every lowered future/task. `poll` returns
  `AwaitStatus` (`Pending` = 0, `Ready` = 1).
- `RuntimeContext` threads executor state to poll/drop. Codegen materialises a hidden `__async_ctx`
  local so await/yield terminators can pass the context pointer.
- `AwaitStatus` governs control-flow branching for both backends.

| Symbol | Behaviour |
| ------ | --------- |
| `chic_rt_async_register_future(FutureHeader*)` | Interns the header, caches an `ExecutorNode`, and back-links `executor_context` so await/yield can find the current task. |
| `chic_rt_async_spawn(FutureHeader*)` | Marks the node runnable and schedules it on the executor (native queue / WASM queue). |
| `chic_rt_async_block_on(FutureHeader*)` | Drives a root future to completion; startup + `Std.Async.Runtime.BlockOn` rely on it. |
| `chic_rt_await(RuntimeContext*, FutureHeader*)` | Polls the awaited future, returning `Pending` when invoked from a runtime-managed task and registering the caller as a waiter; falls back to a blocking poll loop when no context is available. |
| `chic_rt_yield(RuntimeContext*)` | Yields cooperatively only when a runtime context is present; otherwise returns `Ready`. |
| `chic_rt_async_cancel(FutureHeader*)` | Sets `Cancelled|Completed|Ready`, wakes waiters, and returns `Ready` so codegen can continue down the ready branch. |
| `chic_rt_async_task_result(src, dst, len)` | Memcpy-copies a task result into a caller-provided buffer; MIR layouts drive the offsets for `Task<T>.InnerFuture.Result` (and the WASM executor now trusts that metadata when exports are stale). |
| `chic_rt_async_token_{new,state,cancel}` | Minimal cancellation token surface (`*mut bool`) used by `Std.Async.CancellationToken{Source}`; `state` returns `1` when cancelled. |

## Stdlib facade

`packages/std/src/async.ch` exposes `Task`, `Task<T>`, `Future<T>`, and the `Std.Async.Runtime` helpers:
`Spawn` schedules a task, `BlockOn` drives it to completion, and `Cancel` forwards to the runtime
cancel hook. `RuntimeExports.TaskHeader` provides the pointer ABI used by startup and codegen;
`TaskBoolResult` / `TaskIntResult` provide specialized result carriers while generic awaits copy results via
`chic_rt_async_task_result`. `CancellationTokenSource`/`CancellationToken` wrap the runtime
token exports so user code can request cancellation without touching raw pointers. Async `Main`/
`testcase` entry points are surfaced through the same facade, keeping startup/test runners on the
executor path instead of the MIR interpreter.

## Layout metadata

- MIR now synthesises layouts for `Std.Async.Future<T>` and `Std.Async.Task<T>` (including dotted
  aliases) so LLVM/WASM backends can project `Header`, `Flags`, `Completed`, `Result`, and
  `InnerFuture` without hard-coded offsets. With the default 64-bit pointer model:
  - `FutureHeader` starts at offset `0`; `Flags` sits at `24` bytes.
- `Future<T>` places `Completed` at `32` bytes and `Result` at `36` (aligned to `T`).
- `Task<T>` mirrors the base header/flags then aligns `InnerFuture` at `40` bytes.
- Async lowering consumes this metadata to populate ready tasks (vtable + flags), load results, and
  call `chic_rt_async_task_result` with backend-accurate offsets; wasm32 and LLVM share the
  same MIR-driven layout data. WASM additionally probes exported result helpers when present, but
  now falls back entirely to MIR layouts for generic `Task<T>` so async CLI fixtures never skip
  when exports drift.

## Executor behaviour

- **Native:** `AsyncExecutor` keeps a map of `ExecutorNode`s keyed by header pointer. `await`
  inside a managed task registers the current node as a waiter on the awaited future and returns
  `Pending`; ready paths load results and continue. Cancellation marks the node cancelled, sets
  `Completed|Ready`, and wakes registered waiters. When compiled code runs without an executor
  context (e.g., stubbed poll/drop bodies), `chic_rt_await` uses `block_on_without_context`
  to guarantee forward progress.
- **WASM:** The executor bridge implements the same ABI: `chic_rt.await`/`yield` return
  `AwaitStatus`, enqueue awaited tasks, and resume waiters from a ready queue without
  busy-waiting. Cancellation and result-copy helpers share the same flag semantics, and token
  helpers allocate linear-memory booleans. Null/absent contexts still make progress by polling and
  returning `Ready`. Result handling now trusts MIR layout offsets for `Task<T>.InnerFuture.Result`
  and tolerates stale/missing exports by reading directly from linear memory.

## Operational constraints & CLI integration

- Async state machines still rely on minimal poll/drop bodies; the runtime handles missing
  vtables by marking headers ready and falling back to blocking polls so binaries complete even
  before full async lowering lands.
- CLI override wiring mirrors the runtime contract: when `CHIC_SKIP_STDLIB=1` plus
  `CHIC_ASYNC_STDLIB_OVERRIDE`/`CHIC_STARTUP_STDLIB_OVERRIDE` are set, `chic run/test` injects stub
  async/startup modules but still executes on LLVM/WASM using the recorded Task/Future layouts—no
  skips are expected now that generic `Task<T>` projection is supported end-to-end. Native-mode
  test execution falls back to the MIR interpreter if the native executor stub returns success so
  async testcases report pass/fail instead of skipping.
- Native startup routes async `Main`/`testcase` through `Std.Async.Runtime.BlockOn`, and the same
  facade underpins the async CLI fixtures under `tests/testdate/async_*`
  (success/failure/cancellation/timeout).

## Integration coverage

- `cargo test runtime::async_runtime::tests` exercises scheduler transitions, await/yield fallbacks,
  cancellation, and token helpers.
- `cargo test runtime::wasm_executor::executor::bridge::tests::await_future_pending_then_completes`
  plus neighbouring WASM executor tests cover ready/pending/cancel/error propagation and
  import/export handling.
- `cargo test --test backend_validation -- --nocapture` drives CLI async programs on LLVM/WASM with
  stdlib overrides, including cancellation/timeout fixtures and skip diagnostics.

## Current limitations

- Poll/drop lowering is still stubbed; full state-machine rewriting (Tasks 1.17/2.7) will remove the
  blocking fallback and generate native poll/drop glue.
- Cancellation scopes are shallow: tokens are raw booleans without structured propagation or
  destructor hooks, and drop glue for partially-completed frames is not yet synthesised.
- Executors run in single-threaded mode today; a multi-threaded native runtime and executor
  selection hooks will follow once async trait/generator lowering lands.
