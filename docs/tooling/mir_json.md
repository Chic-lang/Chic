# MIR JSON Artifact

Chic emits a machine-readable MIR description whenever `chic build --emit mir.json` is
invoked. The goal is to provide deterministic, agent-friendly metadata that mirrors what the
compiler sees after all semantic analysis has completed.

## File Layout

The artifact is UTF-8 JSON with the following top-level shape:

```json
{
  "version": "0.1.0",
  "target": "x86_64-apple-darwin",
  "symbols": [ { ... } ],
  "diag": [ { ... } ],
  "effects": { "random": [...], "network": [...] },
  "profiles": [ { ... } ]
}
```

- `version` is a semantic version that increments whenever the schema changes. Consumers must
  refuse to load unknown major versions.
- `symbols` enumerates MIR bodies (`functions`, `graphs`, `kernels`). Each entry contains the MIR
  locals, body blocks, statements, terminators, and per-local metadata (types, ownership flags,
  tensor shape/layout information, RNG provenance).
- `diag` surfaces non-fatal diagnostics (e.g., shape mismatch suggestions) that may be useful to
  agents even when the build succeeds.
- `effects` summarises the declared effects per symbol (throws, random, measure, network, cancel).
- `profiles` links schedule/profile hashes to the symbols that consume them so schedule planners
  can detect drift.
- `accelerator` records stream metadata `{ "stream_id", "device_id", "memspace", "events": [...] }`
  so schedule planners and profiling tools can correlate enqueue/record/wait ordering across
  backends. WASM stubs currently log ordering only; emitting full metadata is planned.

## Statement Encoding

Every MIR statement/terminator is encoded as:

```json
{ "stmt": "TensorAlloc", "loc": "src/model.cl:42", "fields": { "...": "..." } }
```

- `stmt` or `term` is the discriminant.
- `loc` is a human-readable span (`file:line[:column]`). Agents should treat it as a hint, not a
  canonical identifier.
- `fields` mirrors the Rust struct fields for that MIR node (e.g., `shape`, `layout`, `memspace`,
  `stream_id`). Enumerations are emitted as strings.

## Determinism Guarantees

- Entries are sorted by symbol name, then block index, then statement index.
- No whitespace other than `\n` line terminators is significant.
- Floating-point metadata (e.g., cost estimates) is emitted with fixed precision matching the
  compiler’s internal representation.

## Schema Evolution

- Minor releases (`0.x`) may add new fields but never remove or rename existing ones.
- Major releases bump the version when incompatible changes occur; clients must opt in.
- Future work will publish a JSON Schema document and a differential tooling harness so agents can
  validate their consumers automatically.

## Related Artifacts

- `docs/tooling/hints_json.md` – compile-time obligations, fix-its, and guidance.
- `docs/tooling/perf_json.md` – deterministic performance traces keyed by MIR instruction IDs.
