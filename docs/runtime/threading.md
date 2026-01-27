# Threading Runtime Notes

Status: In-flight (typed-pointer context validated)  
Owners: Runtime/bootstrap team

## Context ABI

- `ThreadStartDescriptor.Context` is a `ValueMutPtr` sized/aligned for `__StdSyncArcHandle`. Thread
  spawns therefore carry size/alignment metadata with the pointer instead of erasing provenance into
  `usize`. The stdlib validates pointer/size/align before handing the context to the runtime.
- Runtime callbacks `chic_thread_invoke` / `chic_thread_drop` now take the full
  `ValueMutPtr` context and will reject null or mis-sized handles before dispatching to
  `Arc<T>.FromRaw`/drop glue, ensuring typed-pointer parity between LLVM and WASM.
- The callbacks are exported from `Std.Platform.Thread.RuntimeCallbacks` and imported by the minimal
  native runtime; the C/LLVM layer no longer implements any thread payload semantics.
- The runtime adapter checks the `ValueMutPtr` layout against `ChicArc` so invalid contexts are
  rejected with `ThreadStatus::Invalid` rather than leaking or double-dropping.
- WASM backends currently return `ThreadStatus::NotSupported` and drop the context immediately so the
  typed handle never leaks; native backends propagate the handle into OS threads.

## Standard Library Surface

- `ThreadStartFactory.From`/`Function` wrap payloads in `Arc<T>` and hand the typed handle to the
  runtime spawn entrypoint.
- `Thread::Spawn`/`ThreadBuilder::Spawn` clone the payload Arc, build the `ValueMutPtr` context, and
  drop it via runtime drop glue when spawn fails. `Join`/`Detach` forward status codes directly.
- Spin/yield/sleep remain thin shims over the runtime exports so LLVM/WASM share the same ABI.

## Testing & Harnesses

- `tests/runtime_sync_cl.rs` exercises Arc/Rc downgrade/upgrade plus thread primitives under the
  typed-handle model, and runtime unit tests assert that contexts carry the expected size/alignment.
- Concurrency litmus programs (`tests/concurrency/litmus/*.ch`) spawn threads through
  `ThreadStartFactory` and therefore validate the ValueMutPtr context path across the compiler,
  runtime, and OS threads.

## Spec links

- `SPEC.md#threading-synchronisation-runtime` documents the typed
  `ThreadStartDescriptor.Context` contract and runtime callbacks.
- `docs/runtime/arc.md` and `docs/runtime/weak_pointers.md` cover the Arc/Weak raw-handle APIs used
  by the thread runtime.
