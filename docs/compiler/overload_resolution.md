# Chic Overload Resolution Notes

This document captures the current state of Chic’s overload infrastructure across the parser, semantic analysis, MIR builder, and backends. Open work and follow-ups should be tracked as GitHub issues; the spec and user-facing guide are the source of truth for behavior.

## Parser & AST Inventory

- `Module::overloads` (`src/frontend/ast/items/base.rs`) owns an `OverloadCatalog`. Every parser entry point now routes through helpers like `Module::with_namespace_items` so the catalog is rebuilt deterministically after each mutation, keeping symbol tables and tests aligned.
- `src/frontend/ast/overloads.rs` enumerates overload sets for free functions, methods, extensions, impls, and constructors. Each `OverloadEntry` records:
  - Canonical symbol name (`Demo::Vec3::Length`), declaration span, and owning item indices for later lookups.
  - Signature metadata used during resolution: parameter modes (`value`, `in`, `ref`, `out`), whether defaults exist, captured attributes (`@inline`, `@inject`, etc.), async/unsafe modifiers, and generic parameter lists.
  - Accessibility info derived from `Visibility` plus declaring namespace or type.
- Parser and AST unit tests (`src/frontend/ast/overloads.rs`) now assert that free functions, impl/extensions, and constructors all produce accurate overload entries with modifier/default metadata so downstream passes can rely on the catalog.

## Type Checker & Registry

- The type checker consults overload metadata as soon as it sees a call expression. `collect_call_candidates` computes a `CallCandidateSet` from the resolved callee, enclosing namespace, and current `Self` type, filtering out candidates whose visibility would make them inaccessible in the current scope.
- `resolve_overload_for_call` applies the same applicability rules used later by MIR: matching parameter modifiers, ensuring the supplied argument count falls between required and total parameters (allowing defaults), and requiring exact type matches (implicit conversions remain future work). It ranks winners by the number of explicit arguments consumed so that overloads that only succeed via defaults lose to ones that bind more call-site inputs.
- Diagnostics surface via TCK codes:
  - `[TCK141]` – missing required argument (“no overload ... matches”).
  - `[TCK142]` – ambiguous call (“call to `Foo::Bar` is ambiguous”).
  - `[TCK131]/[TCK132]` – constructor-specific no-match/ambiguous errors reuse the same machinery.
- Constructor overloads reuse the shared path by querying `SymbolIndex::constructor_overloads`, ensuring accessibility and default arguments are enforced consistently.
- `typeck::arena`’s diagnostics suite (`src/typeck/arena/tests/diagnostics.rs`) exercises positive and negative overload calls so regressions are caught before MIR building begins.

### Cross-Function Inference Guard (relaxed)

- The simplicity charter still prefers local inference, but the strict guard that rejected `let`/`var` bindings inheriting their type from a call is **relaxed** for now to keep the standard library and stubs concise. The compiler currently accepts bindings such as `var total = Add(1, 2);` without emitting `[TCK146]`.
- When teams want the stricter mode, add explicit casts or type annotations at the call site; `[TCK146]` remains reserved for that enforcement path when it is re-enabled.
- Regression coverage in `typeck::arena::tests::diagnostics::inferred_call_binding_reports_tck146` now asserts the relaxed behaviour, and related registry tests document the current stance.

## MIR Builder & Borrow Checking

- `BodyBuilder::resolve_call_symbol` (`src/mir/builder/body_builder/expressions/calls.rs`) now emits the exact overload selected by type checking. It canonicalises constructor calls, reconciles implicit receivers, validates modifiers for positional and named arguments, and rewrites the `Terminator::Call` to reference the overload’s `internal_name`.
- Tie-breaking mirrors the type checker, so ambiguity and no-match errors only appear in lowering when type checking is skipped (e.g., intentionally malformed modules inside MIR-focused tests). `CallBindingInfo::canonical_hint` threads the selected symbol into borrow checking so lifetime inference and trait obligations observe the concrete overload body.
- Regression coverage in `src/mir/builder/tests/overloads.rs` spans failure paths plus success cases for receiver-vs-static dispatch, defaulted parameters, and constructor overload binding.

## Codegen Backends

- MIR functions carry their overload-specific `internal_name`, and monomorphisation preserves it so LLVM/WASM backends can differentiate overload bodies even when the canonical display name matches.
- LLVM emission (`src/codegen/llvm/emitter/function/tests/overloads.rs`) asserts that call sites invoke the correct mangled symbol (e.g., `Demo__Math__Combine` vs. `Demo__Math__Combine#1`) so inline caches, vtables, and constructor thunks remain deterministic.
- The WASM function emitter (`src/codegen/wasm/tests/function_emitter/overloads.rs`) performs the same verification at the index level, ensuring dispatch tables call the intended ordinal for each overload.

## Tooling Docs & Integration

- Spec §2.22 now documents the overload model (applicability, ranking, constructors, diagnostics) and links back to these implementation notes.
- `docs/guides/overloads.md` summarises overload behavior and common fixes for ambiguous calls.
- CLI integration tests (`tests/cli_diagnostics.rs`) cover both successful execution with overload selection (`run_executes_overloaded_program`) and failure diagnostics (`check_reports_overload_ambiguity`). The LSP harness (`tests/lsp_overloads.rs`) validates that `--log-format json` emits machine-readable overload errors so IDE clients surface consistent messages.

## Testing & Coverage Snapshot

- Parser/AST: `src/frontend/ast/overloads.rs` tests free, method, extension, and constructor catalogues.
- Type checker: `typeck::arena` diagnostics cover missing arguments, ambiguous calls, and constructor overload errors (TCK131/TCK132/TCK141/TCK142).
- MIR builder: `src/mir/builder/tests/overloads.rs` exercises positive/negative call binding, receiver/static dispatch, and constructor resolution.
- Codegen: LLVM and WASM emitters have dedicated overload tests ensuring symbol names/indices remain distinct.
- Tooling: CLI (`tests/cli_diagnostics.rs`) and LSP (`tests/lsp_overloads.rs`) integration tests prove overload diagnostics reach developers in both terminal and editor workflows.
