# Impact Language Access Control

The Chic toolchain now recognises the full set of C#-style access modifiers and enforces
their semantics during lowering. This document summarises the available modifiers, the scopes they
govern.

## Supported Modifiers

| Modifier | Applies To | Meaning |
| --- | --- | --- |
| `public` | All declarations | Visible to every module and backend target. |
| `internal` | All declarations | Scoped to the current Impact module (assembly). |
| `protected` | Class members, nested types | Accessible to the declaring type and any derived type. |
| `private` | Namespace and type members | Restricted to the declaring type. Namespace-level `private`
  declarations are effectively file-internal and cannot be referenced by sibling items. |
| `protected internal` | Class members, nested types | Union of `protected` and `internal`. Usable either from
  derived types or any code in the current module. |
| `private protected` | Class members, nested types | Intersection of `protected` and `internal`. Only derived
  types *inside* the current module may access the member. |

> **Note:** Structs cannot be inherited, so `protected` on struct members is rejected during lowering.

## Enforcement in the Compiler

Lowering now collects visibility information for every declared type and validates type references
while computing layouts and signatures:

- Function and method signatures are rejected if they expose an inaccessible type in their return
  type or parameter list.
- Field and union member layouts trigger diagnostics when they reference private or protected types
  that fall outside the current scope.
- Base-type clauses (`class Derived : Base`) are checked to ensure the derived type is allowed to
  see the specified base.
- Extension targets must be visible from the module issuing the extension.

Diagnostics cite the offending declaration and state the visibility that blocked access, e.g.
`field 'Value' references inaccessible type 'Access::Hidden' (private)`.

## Testing Recommendations

- Keep unit tests that assert positive coverage for `protected internal` and `private protected`
  members alongside negative fixtures that prove the compiler emits diagnostics when scopes are
  violated.
- When adding new type-level features (e.g. nested types, generic constraints), ensure the lowering
  pass records visibility so that access checks remain accurate.

For more detail on the lowering pipeline, refer to `SPEC.md` and `docs/mir_design.md`.
