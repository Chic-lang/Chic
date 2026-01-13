# Memory Space Semantics

Describes the semantics of Chic `MemSpace` types (Host, PinnedHost, Gpu<ID>, Unified) used by
tensors, streams, and regions.

## Definitions

- **Host:** Regular pageable CPU memory; safe for CPU-only access.
- **PinnedHost:** Page-locked host memory suitable for DMA transfers.
- **Gpu<ID>:** Device-local memory for accelerator `ID`.
- **Unified:** Coherent memory shared between host and device (when supported).

## Rules

- Borrowing across memory spaces requires explicit transfers (`EnqueueCopy`).
- Pinned memory must be freed after associated events complete; borrow checker enforces this.
- Unified memory still obeys exclusivity rules; overlapping mutable borrows are forbidden.
- Runtime adapter enforces explicit borrows: pinned/unified buffers track an active borrow and
  reject additional borrows or frees while in use. Dropping the borrow guard releases the buffer.
- Deterministic errors: attempting to free while borrowed returns `BorrowActive`; accessing a freed
  buffer returns `Freed`.

## Tooling

- `mir.json` includes `memspace` metadata per tensor/place.
- `docs/runtime/accelerator.md` and `docs/runtime/region.md` describe allocator behaviour for each
  space.
