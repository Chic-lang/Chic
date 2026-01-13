# Chic Compiler Architecture Guide

This guide captures the ownership expectations and layering rules that govern the Chic bootstrap compiler.

## Module Ownership

- **Frontend (`src/frontend`)** — owned by the *Frontend Working Group*. Responsibilities: lexing, parsing, macro expansion, attribute handling. Frontend modules may depend on `syntax`, `support`, and diagnostics utilities, but never on MIR or backend crates.
- **Syntax & Support (`src/syntax`, `src/support`)** — owned collectively; provides reusable AST helpers, visitors, and shared diagnostics plumbing. These modules expose pure data structures or side-effect-free helpers consumable by both frontend and middle-end layers.
- **Middle-End (`src/mir`, `src/typeck`)** — owned by the *MIR/Analysis Working Group*. Responsibilities: MIR lowering, borrow checking, type checking, const-eval. May depend on frontend AST types, diagnostics, and runtime metadata definitions, but must not import backend code.
- **Backend (`src/codegen`)** — owned by the *Backend Working Group*. Contains backend-agnostic orchestration plus target-specific submodules (`llvm`, `wasm`, `cc1`). Backends can consume MIR structures, type-layout tables, and runtime metadata, but may not call back into the frontend or type checker.
- **Runtime (`src/runtime`)** — owned by the *Runtime Working Group*. Exposes executor, libc-style support, and Impact runtime hooks. Runtime modules may reference MIR data only for metadata consumption and must not import backend code.
- **Driver & CLI (`src/driver`, `src/cli`)** — owned by the *Tooling Working Group*. Responsible for orchestration, diagnostics wiring, and CLI UX. Driver code is permitted to touch every layer but should prefer trait-driven boundaries (e.g., `WasmExecutor`) rather than direct module references.

Each module tree keeps its own `mod.rs` (or `mod` directory) that re-exports the public surface. Internal helpers should remain `pub(crate)` unless explicitly required by another crate component.

## Layering Rules

- Frontend → (Syntax/Support) → MIR/Typeck → Codegen → Runtime. Higher layers may depend on lower layers but not vice versa.
- Diagnostics flow upwards: diagnostics structs live with the producing stage, but rendering happens in the driver/CLI layer.
- Cross-layer data must move through typed contexts or traits (e.g., `TypeckQueries`, `ModuleLoweringDriver`, `WasmExecutor`). Avoid global state or singletons.
- Shared utilities belong in `src/support` or well-scoped submodules; avoid copy-pasting helpers between layers.
- Tests for a given layer live alongside that layer. End-to-end smoke tests may live under `src/codegen` or `tests/` but should import only the public APIs that the layer exposes.

## Keeping docs current

When modifying ownership boundaries or adding new modules:

1. Update this architecture guide with the new owner/layer mapping.
2. Capture significant decisions in the PR description and (when helpful) add a short design note under `docs/`.
