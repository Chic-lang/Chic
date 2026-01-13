# Attribute Grammar Guide

The attribute grammar is split by concern so builtin parsing stays testable and coverage-driven instead of living in a single monolith.

## Module layout

| Module | Scope | Notes |
| --- | --- | --- |
| `grammar.rs` | Orchestrator that delegates to focused modules and records macro/builtin attributes. | Keeps the top-level small; doc-commented entry points only. |
| `grammar::layout` | Layout/MMIO hints (`@StructLayout`, `@mmio`, `@repr`, `@align`). | Owns numeric parsing, layout kind handling (dot or `::` separators), duplicate detection. |
| `grammar::codegen` (`externs`, `strings`, `inline_attr`, `vectorize`) | Backend knobs (`@extern`, `@link`, `@cimport`, `@inline`, `@vectorize`, `@intrinsic`, `@export`, `@global_allocator`, plus crate-level `#![no_std]`). | Centralizes string literal handling and ABI/flag validation. |
| `grammar::diagnostic` | Markers (`@pin`, `@thread_safe`, `@shareable`, `@copy`, `@flags`, `@fallible`). | Emits duplicate/conflict diagnostics instead of silently overriding flags. |
| `grammar::shared` | Shared combinators (`parse_attribute_name`, KV parsing, raw argument splitting, token capture). | Reused by the attribute families and MMIO helpers. |

## Supported attributes

- **Layout & memory**: `@StructLayout(LayoutKind.Sequential[, Pack=<u32>][, Align=<u32>])`, `@mmio(...)`, `@repr(C)`, `@align(<u32>)`. Duplicate layout/MMIO attributes or duplicate `Pack`/`Align` keys emit diagnostics.
- **Codegen/backends**: `@extern([convention="<abi>"][, library="<name>"][, alias="<name>"][, binding="<lazy|eager|static>"][, optional=<bool>][, charset="<name>"])`, `@link("<library>")`, `@cimport("<header>")`, `@export("<symbol>")`, `@inline(local|cross)`, `@intrinsic`, `@vectorize(decimal)`, `@hot`, `@cold`, `@always_inline`, `@never_inline`, `@global_allocator`, plus crate-level `#![no_std]`/`#![std]`.
- **Conditional & optimisation**: `@cfg(<expr>)` prunes inactive items/statements using the `#if` expression grammar; optimisation hints above thread through MIR into LLVM/WASM attributes. Invalid or conflicting hints emit diagnostics instead of silently overriding behaviour.
- **Diagnostics & semantics**: `@pin`, `@thread_safe` / `@not_thread_safe`, `@shareable` / `@not_shareable`, `@copy` / `@not_copy`, `@flags`, `@fallible` (no arguments). Conflicting or duplicate markers always report diagnostics.
- **Macros**: Everything else is treated as a macro attribute; `collect_attributes` preserves raw tokens for macro expansion while marking builtins as non-expandable.

## Fixtures, tests, and coverage

- Parser fixtures live in `src/frontend/parser/tests/grammar/attributes.rs`:
  - `layout_fixture`, `codegen_fixture`, `diagnostic_fixture`, and `parser_fixture` build focused sources for struct layout, codegen knobs, diagnostics, and helper-level parsing.
  - Tests cover both success paths and diagnostics for each submodule (layout/codegen/diagnostic/shared).
- Run the targeted suites and coverage:

```
cargo test --lib frontend::parser::tests::grammar::attributes
cargo llvm-cov --lib --json --output-path coverage/parser_attributes_local.json -- --test-threads=1 attributes::
```

- Coverage targets: ≥85% for all attribute grammar modules.

## Extending the grammar

1. Pick the owning module (layout/codegen/diagnostic); add a `handle_*` branch with duplicate/conflict diagnostics.
2. Use shared helpers for name/value parsing; prefer adding helper utilities to `grammar::shared` instead of inlining.
3. Add parser/regression tests in `src/frontend/parser/tests/grammar/attributes.rs` (success + diagnostics) and rerun the coverage command above.
4. Keep tests and fixtures discoverable so future contributors know where to extend the suite.
