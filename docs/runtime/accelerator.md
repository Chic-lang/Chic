# Accelerator Runtime Model

This document captures the runtime surfaces that back Chic’s accelerator and stream model
(spec §16.5).

## Core Abstractions

- `Stream<M: MemSpace>` – linear capability representing a submission queue.
- `Event<M>` – completion token tied to a specific device/memory space.
- `MemSpace` implementations: `Host`, `PinnedHost`, `Gpu<ID>`, `Unified`.

## Responsibilities

- Native adapters wrap CUDA/HIP/Metal/Vulkan queues; WASM emits deterministic stubs/logs.
- `Stream` borrows must be unique during submission; borrowed tensors outlive outstanding events.
- Pinned host allocations guarantee that DMA transfers remain valid until the event completes.
- Native stream operations return deterministic error codes:
  - `ACCEL_INVALID_STREAM` / `ACCEL_INVALID_EVENT` for mismatched handles,
  - `ACCEL_DEVICE_UNAVAILABLE` when the selected device is offline,
  - `ACCEL_ENQUEUE_FAILED` for rejected submissions (profiling still records the attempt).

## Tooling Hooks

- Every enqueue/record/wait operation records `(stream_id, device_id, span)` in `mir.json`.
- `perf.json` captures overlap statistics so schedule planners can diagnose under-utilisation.
- A lightweight stream log is emitted under `profiling/accelerator/streams.json` describing the
  schema consumed by the agents; the runtime adapters populate the same fields when tracing is
  enabled.
- Stream ordering is deterministic: submissions appear in MIR order unless an explicit event
  dependency reorders them. The mock driver used in unit tests logs every enqueue/copy/record/wait
  to assert the exact sequence without requiring a device.

## Async Behaviour

- Async functions that capture `Stream<M>`/`Event<M>` must pin those locals (`@pinned` or by using
  `PinnedHost` memory spaces). The borrow checker rejects unpinned captures across `await` and
  threads buffer borrows through completion events until a matching `wait`.
- `WaitEvent` releases the queued borrows so buffers can be reused safely; omitting the wait
  surfaces a borrow error on the first move/drop.
- The LLVM backend records stream metadata; the WASM backend runs the same tests through stubbed
  accelerator hooks to keep ordering deterministic.

## Metadata & Diagnostics

- `mir.json` includes per-stream metadata `{ "stream_id", "device_id", "memspace", "events": [...] }`
  so planners and profiling tools can correlate submissions across backends.
- Unsupported copies (layout/memspace mismatch) surface `Codegen` diagnostics instead of inserting
  hidden staging buffers. The runtime adapters mirror this by logging the failed attempt when no
  accelerator is present.
- GPU launches (stub): `runtime_adapter/native/gpu_launch.rs` will validate grid/block dimensions
  and enforce typed parameters for `@gpu_target` kernels. Streams/events are threaded through the
  same ordering guarantees documented above.

## Open Questions

- Multi-device synchronisation primitives (semaphores) will be addressed in a follow-up.
- Integration with the memory planner to reserve staging buffers ahead of time.
