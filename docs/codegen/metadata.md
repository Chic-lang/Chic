# Chic Metadata Emission

The metadata subsystem emits auxiliary objects that decorate Chic artifacts with
build provenance (target triple, ChicKind), exported symbol maps, documentation payloads, and
telemetry used by the LLVM/WASM backends. The subsystem lives under
`src/codegen/metadata/` and is split into focused modules so backends can re-use building blocks
without duplicating logic.

## Module Layout

- `mod.rs` orchestrates metadata emission. It exposes `write_metadata_object` for native
  targets, `build_metadata_bytes_with_caches` for cache-aware callers, and aggregates telemetry via
  `MetadataCaches`.
- `attributes.rs` walks the AST to serialise documentation and module-level attributes while
  registering type fingerprints in the cache.
- `functions.rs` normalises export metadata, tracking cache hits/misses for symbol descriptors.
- `types.rs` houses the fingerprinted type cache and telemetry structs shared by both backends.
- `debug.rs` contains target-triple helpers (format mapping, Mach-O build version encoding, section
  naming) without touching IO.

Each module is side-effect free (pure string building / collection) so tests can exercise logic in
isolation.

## Caches & Telemetry

`MetadataCaches` wraps `TypeMetadataCache` and `FunctionMetadataCache`. Both caches expose
`ensure_with` helpers that deduplicate entries and update hit/miss counters. Fingerprints currently
use qualified type names; once layout hashing lands the fingerprint implementation can be swapped
without changing callers.

`MetadataTelemetry` snapshots cache activity for CI reporting and backend diagnostics. The native
backend obtains telemetry via `write_metadata_object`, while other callers can use
`build_metadata_bytes_with_caches` to receive `(Vec<u8>, MetadataTelemetry)`.

Key expectations:

- Type cache records every struct/enum/union/class/interface/extension encountered during doc
  traversal.
- Function cache records every explicit export written into the payload.
- Telemetry should be surfaced in backend reports or CI logs (the LLVM backend now captures it via
  `CodegenArtifact::metadata_telemetry`).

## Backend Integration

- The LLVM backend calls `write_metadata_object` and records the returned path plus telemetry inside
  `CodegenArtifact`.
- Cached compilation rehydrates `CodegenArtifact` via `codegen::cache`; metadata telemetry is not
  stored on disk yet, so cache hits report `None` for telemetry.
- Wasm/cc1 backends currently emit metadata through their own flows (no telemetry). When those flows
  need cache support they can adopt `MetadataCaches` and `MetadataTelemetry` without touching native
  code.

## Testing

`cargo test metadata::` exercises cache accounting, telemetry, and doc traversal. Integration tests
cover `write_metadata_object`, ensuring the emitted object matches the cached payload and telemetry
reports expected hit/miss counts.

Use these tests when modifying metadata emission or adjusting fingerprint logic.
