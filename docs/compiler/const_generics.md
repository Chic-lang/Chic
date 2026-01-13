# Const Generics Authoring Guide

Const generics are now part of the Chic surface across the full pipeline (parser → type checker → MIR → LLVM/WASM). This note captures best practices, diagnostics, and tooling hooks developers should follow when adding const-parameterised APIs.

## 1. Declaring & Naming Const Parameters

- Place `const` parameters before ordinary type parameters when they materially affect layout/ABI, e.g. `struct Buffer<const N:int, T>`. This keeps specialization order consistent with mangling.
- Use uppercase identifiers for const parameters (`LANES`, `ROWS`, `TIMEOUT_MS`) so they stand out from type parameters in diagnostics. The parser doesn’t enforce casing, but internal code reviews and lints look for screaming-snake case.
- Choose the narrowest scalar type that describes the invariant (`bool` for feature flags, `uint` for sizes that cannot be negative, `int` for signed offsets). Wider-than-needed types reduce opportunities for constraint checking.

## 2. Constraints, Evaluation, and Diagnostics

- Guard every unbounded const parameter with at least one predicate in the `where` clause. Use `const(<expr>)` predicates so the type checker can validate them during instantiation:

  ```chic
  public struct Tiles<const LANES:int, T>
      where LANES : const(LANES % 4 == 0 && LANES <= 64)
  { /* ... */ }
  ```

- Both argument expressions and predicates run through the const-eval engine. Failures surface the following diagnostics:
  - **CONST_EVAL_FAILURE** — expression failed to evaluate (syntax error, overflow, division by zero, accessing unknown symbol).
  - **GENERIC_CONSTRAINT_VIOLATION** — predicate evaluated to `false` or returned a non-`bool`.
  - **GENERIC_ARGUMENT_MISMATCH** — wrong number or kind (type vs const) of arguments at an instantiation site.
  - **DUPLICATE_GENERIC_PARAMETER** — repeated const parameter names.
- When adding new diagnostics, reuse these codes where possible so tooling (IDE hints) can recognise them.

## 3. Tooling & Lint Hooks

- Symbol mangling embeds the *normalised* const argument text. You can inspect canonical names via `mir::data::definitions::strings::canonical_ty_name`. If the output doesn't include your const values, ensure the parser is recording them (they must survive through `GenericArgument::set_evaluated_value`).
- Preferred diagnostics workflow:
  1. Reproduce the issue with `cargo test frontend::parser::tests::grammar::generics -- --nocapture` or `cargo test typeck::arena::tests::diagnostics`.
  2. Confirm the emitted message matches one of the codes listed above.
  3. Document tricky patterns (e.g., negative sizes, mismatched const/type argument counts) in regression tests under `src/typeck/arena/tests`.

## 4. Interop & Backends

- MIR, drop-glue synthesis, and both backends treat const arguments as part of the type identity. When adding runtime/interop features (e.g., new stdlib containers), keep const parameters near the front of the generic list to guarantee stable symbols across builds.
- WASM + LLVM smoke tests now include `pipeline_handles_const_generics_across_backends`. Add new fixtures there when working on backend regressions; it produces both LLVM IR and a `.wasm` artifact so you can diff symbol names across targets.

Following these conventions keeps const-generic code consistent with the rest of the compiler and ensures existing tooling (diagnostics, backend smoke tests) continues to pass. If behaviour changes, update both this guide and `SPEC.md` in the same change.
