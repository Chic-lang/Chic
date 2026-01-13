# Memory Planner

The graph memory planner produces deterministic buffer offsets to avoid overlap between tensors
within a region. This stub models sequential packing until richer alias analysis is integrated.

## Data Model

- `MemoryPlan` (Rust: `mir::passes::memory_plan::MemoryPlan`) contains `BufferPlan` entries with
  `{ id, size, offset }`.
- Plans validate non-overlap by ensuring `[offset, offset + size)` ranges do not intersect.
- Runtime adapters expose a helper (`runtime_adapter/native/memory_plan.rs`) for agent tooling and
  tests; WASM will reuse the same schema.

## Behaviour

- Buffers are packed sequentially in deterministic order of appearance; future passes may fold
  lifetimes to alias buffers with non-overlapping usage windows.
- Planning is side-effect free and purely deterministic; no hidden allocations are introduced.

## Tests

- `tests/memory_plan.rs` validates non-overlap and offset monotonicity for a small set of buffers.
