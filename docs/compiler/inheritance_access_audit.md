# Inheritance and access audit

Date: 2025-12-28  
Scope: class inheritance + access modifiers, cross-package/assembly behaviour, abstract/virtual/override enforcement, diagnostics.

## Current vs Spec (observed vs required)

| Rule / Modifier | Spec (C#-like) | Current implementation | Status |
| --- | --- | --- | --- |
| `public` | Always accessible. | Allowed everywhere. | OK |
| `private` | Only declaring type (and nested types, if supported). | Static members check declaring type, but **instance fields/properties/methods have no accessibility check at all**—any caller can read/write/call them. | **SOUNDNESS leak** |
| `internal` | Same assembly/package only. | Type checker uses package identity for inheritance/constructors, but MIR lowering ignores packages (uses namespace root) and instance members skip visibility entirely. Cross-package callers can consume internals. | **SOUNDNESS leak** |
| `protected` | Declaring type + derived types (any assembly) + protected-instance rule. | No enforcement for instance members; any receiver works. Static checks ignore package and protected-instance semantics. | **SOUNDNESS leak** |
| `protected internal` | Union: protected **or** internal. | No member-level enforcement; static path falls back to namespace matching. | **SOUNDNESS leak** |
| `private protected` | Intersection: protected **and** internal (same assembly + derived). | Not enforced for instance members; static path ignores package and uses namespace-only “internal”. | **SOUNDNESS leak** |
| Type accessibility in signatures | Public signatures must not expose less-accessible types; bases/interfaces must be at least as visible. | Only “public type cannot inherit from less accessible base” is checked. Parameter/return/field/property/interface type visibility is unchecked. | **SOUNDNESS leak** |
| Protected-instance rule | Derived code may only use protected members through `this`/derived-typed receivers. | Not implemented; receivers of arbitrary base type are accepted. | **SOUNDNESS leak** |
| Package/assembly identity | Manifest package boundary defines assembly. | Type checker threads package for some checks; MIR/builder has no package awareness, so accessibility devolves to namespace matching or nothing. | **SOUNDNESS leak** |
| Abstract class instantiation | Abstract types cannot be constructed. | No guard in `new` binding or lowering; `new Abstract()` compiles. | **SOUNDNESS leak** |
| Abstract member implementation | Non-abstract derived types must implement inherited abstract members. | Enforced. | OK |
| Override rules | Default sealed; `virtual` enables override; `override` required; `sealed override` stops further overrides. | Enforcement exists but rides on permissive accessibility; needs revalidation once lattice is fixed. | Needs recheck |
| Base calls | `base.Member` only when member exists and is accessible. | Member accessibility gaps apply here too. | Needs recheck |

## Bug list with minimal repros

### SOUNDNESS / SECURITY
- **Instance member visibility ignored**: Any code can access private/protected/internal instance fields/properties/methods because member lowering never checks visibility.  
  - Repro (single package):  
    ```chic
    public class Secret { private int Key; internal void Touch() { } protected int P => 1; }
    public class Snooper { public int Leak(Secret s) => s.Key + s.P; public void Call(Secret s) => s.Touch(); }
    // Compiles; should be rejected (private/protected/internal).
    ```
- **Cross-package leaks (internal/private protected)**: MIR lowering has no package identity; internal/private protected members are callable from dependent packages that share a namespace root.  
  - Repro: Package A `namespace Shared; public class A { internal int Hidden; private protected int Gate; }`. Package B `import Shared; public class B { public int Peek(A a) => a.Hidden + a.Gate; }` succeeds.
- **Protected-instance rule absent**: Derived classes can invoke protected members on arbitrary base-typed receivers.  
  - Repro: `class Base { protected void Ping() { } } class Derived : Base { public void Run(Base other) { other.Ping(); } }` compiles; should fail because `other` is not known to be `Derived`.
- **Abstract instantiation allowed**: `abstract class Ghost { } class User { public Ghost Make() => new Ghost(); }` compiles.
- **Type exposure not checked**: Public members freely expose internal/private protected types in signatures; no diagnostics.

### Correctness / Diagnostic noise
- Lack of access filtering causes overload resolution to consider unreachable members, leading to cascades or ambiguous matches instead of a single “inaccessible” diagnostic.
- Static member visibility uses namespace roots instead of packages, so unrelated packages with the same root spuriously gain/lose access.

## Fix plan (per bug cluster)
- Introduce a unified accessibility lattice keyed on manifest package identity, receiver static type, and inheritance checks (including protected-instance rule).
- Thread package identity through MIR/module lowering, symbol tables, and type checker; store owner package on types/functions/constructors.
- Enforce accessibility for:
  - Member access (instance + static), base calls, and call overload filtering.
  - Type references in signatures, bases, interface impls, and extension targets.
  - Constructor selection and object initializers.
- Enforce `private protected` as (protected AND internal) and `protected internal` as (protected OR internal) across packages.
- Reject abstract type construction in both type checking and lowering.
- Improve diagnostics: emit a primary “inaccessible” error with notes instead of cascades when visibility blocks resolution.

## Test plan
- **Unit/typeck**: Matrix of all modifiers on fields/methods/properties across same type/namespace, different namespace same package, different package, derived vs non-derived. Include protected-instance allowed/denied cases, type-exposure failures, abstract `new`, and override/base-call legality.
- **Multi-package integration**: Two-package workspace where Package A defines bases with every modifier; Package B (non-derived and derived) attempts access. Assert internal/private protected stay hidden cross-package; protected internal only via protected path; protected obeys instance rule; protected internal/override/base calls behave.
- **Diagnostics**: Scenarios where only accessibility should fail (e.g., overload set filtered out) yield single targeted diagnostics without ambiguity spam.

## Notes / open questions
- Nested-type visibility is still undefined in the implementation; keep scope limited to top-level types unless the spec requires nesting semantics.
- Header/metadata emission should mirror the final lattice (public surface only); revisit once enforcement is in place.
