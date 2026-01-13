# Graph Lowering Pipeline

This note captures the lowering strategy for Chic graphs and schedules across LLVM and WASM backends. Goals: deterministic ordering, no hidden allocations, and stable metadata for schedule/profile consumers.

## MIR Representation

- Graph bodies use `GraphBegin`/`GraphEnd` markers with `GraphNode { id, op_kind, inputs, outputs }`.
- Schedule directives attach to graph metadata rather than emitting statements. Profile hashes travel alongside the schedule metadata.
- Deterministic ordering: nodes execute in topological order as emitted by MIR; explicit dependencies remain encoded for schedulers to reorder legally.

## Lowering Strategy

- LLVM: translate graph nodes into backend launch sequences (kernel calls or function calls). Events/streams use the same deterministic ordering as accelerator lowering. No runtime codegen is allowed; all launch parameters are materialised in MIR.
- WASM: emit linear-memory dispatch loops that walk the graph nodes in order, invoking lowered functions directly. Schedules affect loop tiling/unrolling metadata only.
- Metadata: each lowered graph exports `{ graph_id, nodes, schedule, profile_hash }` so tools can validate hash matches at build time.

## Registry & Profiles

- `graph_registry` maps graph IDs to lowered launch sequences for both backends.
- Profile sidecars (`profiling/profiles/*.json`) include a hash and tuned parameters; mismatches should fail the build via `profile_loader`.

## Testing

- Snapshot tests should cover scheduled graphs for both backends, asserting deterministic ordering and emitted metadata.
- Profile loader tests ensure hash mismatches surface deterministic diagnostics.
