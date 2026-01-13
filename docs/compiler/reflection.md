# Reflection metadata layout

Chic reflection is generated at compile time and shipped in two parallel forms:

- **In-memory descriptors:** the frontend builds deterministic `ReflectionTables` sorted by `FullName`. Each type row carries a `TypeId` (`blake3` hash of the full name), ordered member/parameter/attribute lists, and an ABI-backed `TypeLayoutDescriptor`. Arrays are represented as `DescriptorList<T>` in CTFE to avoid allocations; the same ordering is preserved when serialising.
- **Embedded blobs:** the compiler serialises the tables to UTF-8 JSON and emits them into `@__chic_reflection` (LLVM) or `chic.reflect` (WASM) sections. The blob mirrors `<artifact>.reflect.json` (`{ "version": 2, "types": [...] }`) and is also bundled into `.clrlib` as `metadata/module.reflect.json`.
- **Runtime entrypoints:** `Std.Meta.Reflection.reflect<T>()` resolves directly to the canonical `TypeDescriptor` for `T` (no allocation, O(1), identical on LLVM/WASM). `typeof(T)` returns the corresponding `TypeHandle` so code can compare `TypeId`/`Name` without materialising the descriptor.

## Accessor representation

Field/property accessors are optional and only generated when safe under Chic's borrowing rules:

- Read accessors use either typed `ref` shims or raw-pointer thunks (`fn(void* instance, void* out_value)`). Write accessors require a mutable/`ref` receiver and are never generated for `readonly` or init-only members.
- Generic, trait, and interface members only gain accessors when the underlying target is monomorphised and layout-stable.
- Metadata-only builds leave accessor slots empty in both the JSON and the embedded blobs; callers must use static typing or layout metadata instead of boxing values.
