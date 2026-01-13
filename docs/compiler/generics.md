# Generic Parameter Variance & Metadata Plumbing

This note records the implementation details behind §3.64 “Generic Constraint
Coverage & Variance”. It complements the normative spec text in
`SPEC.md` by outlining how the compiler, metadata
writers, and runtime surface the declared variance for every generic type.

## Syntax & AST

- `in` / `out` modifiers are accepted only on interface (and future delegate)
  type parameters. The parser emits `TCK022` when a modifier appears on a class,
  struct, enum, trait, or const generic.  
- `GenericParam::type_param` carries a `Variance` enum; the parser stores the
  value so downstream stages can inspect it.  
- Delegates reuse the same syntax once the delegate AST lands (tracked under
  §3.13). Until then the parser rejects variance on delegates and reports that
  the feature is pending.

## Symbol Index & MIR

- `SymbolIndex::record_type_generics` snapshots the variance for every generic
  type while walking the AST.  
- During lowering, the index drains into `MirModule::type_variance`, producing a
  `HashMap<String, Vec<TypeVariance>>` keyed by fully-qualified type names.
  Classes remain invariant, whereas interfaces inherit their declared modifiers.  
- The MIR builder tests added in §3.64 Task C (`mir::builder::tests::variance`)
  enforce the mapping:

  ```
  cargo test mir::builder::tests::variance::type_variance_records_interface_annotations
  ```

## Metadata & Reflection

- `collect_reflection_tables` now prints variance keywords inside the `generics`
  descriptor field (for example `out TResult`, `in TArgument`).  
- `synthesise_type_metadata` copies the `TypeVariance` vectors into each
  `SynthesisedTypeMetadata` entry so LLVM and WASM emit identical byte streams.
  Runtime installers expose the values via `RuntimeTypeMetadata.variance`.
- `src/runtime/type_metadata.rs` gained regression tests verifying that
  `VarianceSlice::as_slice` and the runtime registry round-trip variance arrays:

  ```
  cargo test runtime::type_metadata::tests::metadata_lookup_returns_variance_information
  ```

- Reflection coverage asserts that descriptors surface the keywords:

  ```
  cargo test frontend::metadata::reflection::tests::variance_keywords_emit_in_generics_descriptors
  ```

## Tooling & Runtime

- `RuntimeGenericVariance` mirrors the AST enum (`Invariant`, `Covariant`,
  `Contravariant`). Tooling that inspects `RuntimeTypeMetadata` can compare two
  instantiated interfaces safely by reading the variance byte stream written by
  the backends.  
- CLI tooling (reflection dumps, metadata viewers) continues to rely on the
  descriptor strings; the variance keywords keep those outputs stable across
  backends.

## Auto-Trait Constraints

- Generic parameters can now opt into Chic’s auto traits directly in their
  constraint lists using `@thread_safe` and `@shareable`. The parser records the
  annotations (`frontend::parser::tests::grammar::generics::parses_auto_trait_constraints`)
  and emits a targeted diagnostic when an unknown attribute appears (see
  `...::rejects_unknown_auto_trait_constraint`).
- `TypeChecker::validate_generic_arguments` evaluates the requested traits for
  every concrete argument. Missing traits raise `[TCK035]`, while unproven
  traits raise `[TCK037]`. When the argument is itself a type parameter, the
  type checker confirms that the surrounding context declares the same
  constraint before allowing the instantiation to proceed.
- The regression suite includes small end-to-end samples under
  `typeck::arena::tests::diagnostics::{auto_trait_constraint_rejects_non_thread_safe_argument,auto_trait_constraint_respected_for_generic_arguments}`
  as well as unit coverage for the async lowering helpers that decide which
  auto trait to emit for `ref`/`ref readonly` locals.

## Regression Matrix

- Parser + AST: `frontend::parser::tests::grammar::generics::parses_variance_on_interface_generics`
- MIR map: `mir::builder::tests::variance::type_variance_records_interface_annotations`
- Reflection: `frontend::metadata::reflection::tests::variance_keywords_emit_in_generics_descriptors`
- Runtime metadata: `runtime::type_metadata::tests::metadata_lookup_returns_variance_information`

Keep this list in sync with new coverage so Task 3.64 stays green in CI.
