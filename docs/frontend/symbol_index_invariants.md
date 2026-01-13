# Symbol Index Invariants

The MIR symbol index now has two layers:

1. `symbol_index/storage.rs` holds the canonical `SymbolStorage` struct. Each map/set is
   keyed by a fully-qualified name (e.g. `Namespace::Type`). Fields/properties are nested
   under their owner key and never mix names between types.
2. `symbol_index/updates.rs` owns all mutation logic. Every helper takes a mutable
   `SymbolIndex` reference and mutates the storage through a single code path so
   invariants stay centralized.

Key invariants enforced when adding/updating symbols:
- `types` only contains fully-qualified names. Nested namespaces are flattened during
  collection so `Namespace::Inner::Type` is canonical.
- `type_methods` counts method overloads per canonical owner. The helper
  `canonical_method_owner` normalizes namespace prefixes so extensions share the same bucket.
- Constant tables (`constants`, `type_constants`, `namespace_constants`) never overlap:
  a symbol lives in exactly one map depending on whether it has an owner or not.
- Field/property dictionaries are keyed by owner + member name. Updates replace existing
  entries atomically so lookups never see partially-updated symbols.
- Extension placeholders are removed as soon as a concrete method registration occurs, so
  extension dispatch never reports stale entries.

When adding new symbol categories, prefer placing the state in `SymbolStorage` and extend
`updates.rs` with helper methods so future changes remain localized.
