# Tooling Notes: Conditional Compilation & Hints

Editors and agents can share the compiler’s conditional logic to keep inactive code dimmed and to surface optimisation hints in generated artifacts.

## Active Configuration

- `src/frontend/conditional.rs` exposes `ConditionalDefines` plus `evaluate_condition_with_diagnostics`; `src/frontend/cfg.rs` applies `@cfg` to AST modules and statements.
- `parse_module_with_defines` (in `src/frontend/parser/mod.rs`) runs `#if` preprocessing and `@cfg` filtering in one call, returning the surviving module plus diagnostics. IDE adapters can reuse it to mirror the compiler’s view of active/inactive regions.
- The define map is case-insensitive and includes `DEBUG`/`RELEASE`/`PROFILE`, `TARGET`/`TARGET_TRIPLE`/`TARGET_ARCH`/`TARGET_OS`/`TARGET_ENV`, `BACKEND`, `KIND`, and `feature_<name>` flags from `--define feature=a,b`. Unknown identifiers evaluate to `false`.
- Attribute errors (missing parentheses, mixed string/bool comparisons, conflicting hints) arrive as regular diagnostics; surface them inline so authors see why code is greyed out.

### Highlighting Guidance

- Treat removed statements as fully inactive; when an `if` loses its `then` arm but keeps `else`, surface the `else` span as the active branch.
- Keep macros opt-in: run `apply_cfg` after macro expansion to reflect generated items, but allow a “pre-expansion” view for authoring ergonomics.
- Offer quick-fixes that add `--define KEY=value` to launch configs when users hover unknown identifiers in conditions.

## Optimisation Hint Surfacing

- MIR carries hint flags on every `MirFunction.optimization_hints` entry. LLVM IR mirrors them via `hot`/`cold`/`alwaysinline`/`noinline` attributes; Wasm modules emit a custom section `chic.hints` with `symbol:hint|hint` entries.
- Profilers or execution harnesses can read `chic.hints` to colour-callstacks or to validate that hot paths stay small. Editors should display hints in function hovers without changing inline/formatting decisions (hints are advisory).
