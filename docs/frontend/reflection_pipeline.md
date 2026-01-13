# Reflection Metadata Pipeline

The reflection pipeline now flows through three focused stages:

1. **Query** (`src/frontend/metadata/reflection/query.rs`) walks the parsed AST and
   materialises `ReflectionTables`. Each helper concentrates on a single AST kind so
   regression tests can exercise them without touching serialization.
2. **Transform** (`src/frontend/metadata/reflection.rs`) exposes the data model and
   orchestrates the public helpers (`collect_reflection_tables`,
   `serialize_reflection_tables`, etc.).
3. **Emit** (`src/frontend/metadata/reflection/emit.rs`) sorts descriptors for
   stability and serialises/deserialises the tables via `serde_json`.

## Golden fixtures

- The canonical fixture lives at `tests/golden/reflection/basic.json` and is consumed
  by `tests/reflection_golden.rs`.
- Update it by running `cargo test reflection_metadata_matches_golden_fixture -- --nocapture`
  and copying the newly printed JSON into the file (or by writing a helper script that
  calls `collect_and_serialize_reflection`).
- Keep the JSON pretty-printed (2-space indent) so diffs stay readable.

## Adding new descriptors

1. Extend the data model in `reflection.rs` if a new field is needed.
2. Update the relevant query helper (e.g., `push_struct` or `property_member`).
3. Add/extend a unit test inside `query.rs` to cover the new case.
4. If the serialized shape changes, re-run the golden test and update
   `tests/golden/reflection/basic.json` to the new output.
