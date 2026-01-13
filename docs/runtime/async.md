# Async Runtime Notes

This short note links to `docs/runtime/async_runtime.md` and captures additional requirements for
structured cancellation scopes (`spec §16.10`).

- For executor architecture, polling strategy, and cooperative scheduling details, see
  `docs/runtime/async_runtime.md`.
- Cancellation tokens:
  - Native/WASM adapters expose deterministic `CancelSource`/`CancelToken` that track budget units
    and optional deadlines (ns). Budget exhaustion or deadline checks mark the token canceled; the
    state is shared across clones.
  - The Std surface (`Std.Async.CancelSource/CancelToken`) mirrors the metadata for codegen and
    layout consistency; runtime adapters drive the actual state machine.
- Structured scopes:
  - `ScopeTracker` (native/wasm) records spawned tasks. `finalize()` fails if any task was neither
    completed nor canceled, enforcing the structured concurrency invariant that no task escapes its
    scope.
  - `cancel_scope` implementations must call `cancel_all()` before dropping the tracker to ensure
    deterministic cleanup when bubbling cancellation outward.
