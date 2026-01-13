# Attribute Pipeline Staging

Updated: 2025-11-02

Chic’s frontend now separates attribute handling into two explicit phases:

1. **Collection (parser)** – `src/frontend/parser/attributes` records every surface attribute as an AST `Attribute`, while tracking builtin flags (`@pin`, `@extern`, `@mmio`, …) required for immediate syntactic checks. No semantic interpretation of DI/module metadata happens in this phase; the parser simply hands back `CollectedAttributes { builtin, list }`.
2. **Expansion (macro expander)** – the macro expander calls `frontend::attributes::stage_builtin_attributes` after macro execution. This staged pass walks the expanded module tree, interprets DI/module annotations via `extract_service_attribute`, `extract_module_attribute`, and `extract_inject_attribute`, and pushes `Diagnostic::error` entries for invalid lifetimes, duplicates, or unsupported arguments. The pass writes the resulting metadata into the AST (`ClassDecl.di_service`, `ConstructorDecl.di_inject`, `Parameter.di_inject`, etc.) so downstream components (DI manifest, type checker, MIR lowering) see the same semantics they relied on previously.

## Operational Notes

- The staged pass is idempotent and always resets `di_*` fields before re-populating them; repeated invocations (e.g., test fixtures and macro expansion) are safe.
- Diagnostics raised during staging are appended to the macro-expander’s output, keeping parse errors and staged errors in a single channel for the driver.
- Parser fixtures (`tests/frontend/parser/tests/fixtures/mod.rs`) run `stage_builtin_attributes` on successful parses to mirror the production pipeline in unit tests.

## Adding New Built-In Attributes

1. Extend `CollectedAttributes::builtin` if the parser must enforce syntactic restrictions.
2. Implement semantic extraction helpers in `frontend::attributes` alongside the DI helpers (returning `(Option<T>, Vec<AttributeError>)`).
3. Update `stage_builtin_attributes` (or a helper it calls) to apply the semantics and push diagnostics.
4. Add parser + staging tests exercising valid/invalid forms.

This division keeps the parser lightweight while ensuring semantic validation happens after macro expansion produces its final AST.

## Async promotion attributes

- `@stack_only`, `@frame_limit(bytes)`, and `@no_capture(move)` are collected by the parser but interpreted during MIR async lowering, where frame-size/capture analysis lives; violations emit **AS0001–AS0003** and malformed usages trigger **AS0004** with spans on the offending attribute.
- Violations surface as **AS0001–AS0004** diagnostics with attribute/local spans; see `docs/runtime/async_runtime.md` for the runtime surface.

## Macro metadata and expansion

- The parser now tags macro-capable attributes with `AttributeMacroMetadata` carrying an `expandable` flag plus the whitespace-free token stream for the attribute span. Builtins keep `expandable = false` but retain tokens for tooling and diagnostics.
- The macro expander consumes that metadata, assigns deterministic hygiene identifiers per pass, strips macro annotations from the AST, and stamps generated code with the originating attribute span. This ensures diagnostics and LSP features can point back to the macro site even after expansion.
- Full ordering, hygiene, and failure rules are documented in `docs/compiler/attribute_macros.md`.
