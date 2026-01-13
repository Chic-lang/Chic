## Metadata Flow

```
Frontend AST + MIR
        |
        v
 populate::build_metadata_payload
   - attributes (docs/inline/profile)
   - functions (exports/extern/default args/lending)
   - layouts (views)
        |
        v
 schema::{MetadataHeader, MetadataWriter}
   - validates fragments (no NUL)
   - assembles header/version/target/kind
        |
        v
 serialize::build_metadata_object_bytes
   - maps target triple -> format/arch via debug.rs
   - writes .chic.meta section + __chic_metadata symbol
   - embeds reflection JSON when provided as __chic_reflection (or platform section alias)
        |
        v
 metadata_object_path(output) -> <artifact>.meta.o
```

### Responsibilities
- `schema.rs`: defines `MetadataHeader`, `MetadataWriter`, and `MetadataPayload` (header + body + NUL terminator). It enforces fragment validation and provides byte/text renderers.
- `populate.rs`: owns `MetadataCaches` + `MetadataTelemetry` and builds the payload body from AST/MIR using attribute/function helpers. Add new metadata here so tests can assert snapshots.
- `serialize.rs`: maps target triples, builds the metadata + reflection sections/symbols, and serialises via `object` (Mach-O build version, segment/section names).
- `debug.rs`: target ↔ format helpers reused by both backends.

### Extending
- Add new metadata lines in `populate::build_metadata_payload`, using `MetadataWriter` for validation. Prefer key=value or `foo:bar=baz` style to keep snapshots readable.
- Record cacheable data in `MetadataCaches` when adding type/function metadata so telemetry remains accurate.
- Update or add tests under `src/codegen/metadata/mod.rs` for new payload lines and serialization branches; snapshots should continue to rely on `metadata_payload` output.
- Provide reflection bytes (UTF-8 JSON) to `build_metadata_object_bytes` if your pipeline has them; section names are handled automatically per format.
- Run targeted coverage: `cargo llvm-cov --lib --json --output-path coverage/codegen_metadata_local.json -- codegen::metadata::tests::`.

### Snapshot Expectations
- Metadata payload text always begins with `Chic Metadata` header lines (version, target-requested, target-canonical, kind) and ends with a single `\0`.
- Section names: Mach-O `__chxmeta` (metadata) and `__chxreflect` (reflection), COFF `.chicxmeta` / `.chicxreflect`, ELF `.chic.meta` / `.chic.reflect`.
- Symbol names: metadata is always `__chic_metadata`; reflection is `__chic_reflection`.
