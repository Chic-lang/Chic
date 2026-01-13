# Traits (compiler notes)

This document summarises how the compiler represents and checks traits. It is intended for contributors working on parsing, type checking, and lowering.

## High-level model

- **Declarations:** `trait` and `impl` items are registered into the symbol index with their members, bounds, and visibility.
- **Resolution:** method calls that involve trait bounds produce trait obligations that are solved during type checking.
- **Diagnostics:** trait failures emit focused codes (missing impl, ambiguity, orphan/coherence violations, and object-safety failures).

## Trait objects

- `dyn Trait` lowers to a `{ data_ptr, vtable_ptr }` pair.
- Vtables are emitted deterministically from the resolved `(Trait, Impl)` pairing and contain method pointers and associated-type metadata.

## References

- Language spec: `SPEC.md`
- User guide: `docs/guides/traits.md`
