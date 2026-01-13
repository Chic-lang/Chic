# Name Resolution Stability Audit (Base classes & FQNs)

Date: 2025-12-28  
Goal: Identify contexts where unqualified vs fully qualified type names resolve inconsistently (especially base/interface lists) and where the compiler bypasses the canonical resolver.

## Current vs expected (C#-like)

| Context | Expected (C#-like) | Observed / suspected gaps | Root-cause bucket |
| --- | --- | --- | --- |
| Base class list `class D : Base {}` | Use normal type lookup (aliases, current namespace chain, imports, implicit Std). FQNs always work. | Base resolution appears to call `resolve_type_info` on a string name captured in `BaseTypeBinding` (already “name” text). If the name was recorded as FQN during parsing, it succeeds; otherwise relies on `type_is_subclass_of_name` and symbol-index names, which are FQN-string based. Imports/alias changes can fail or pick the wrong type. | (4) Internal bypass / FQN string reliance |
| Interface list | Same rules as base class list. | Same path as bases; risk identical to base types. | (4) |
| Type expressions in fields/parameters/returns | Should resolve unqualified via canonical resolver. | Type checker uses `resolve_type_for_expr` which calls `using_resolver` (now `import_resolver`), so mostly correct. | (2) namespace merge issues possible |
| Constructor `super(...)` target | Should resolve base ctor on resolved base type. | Relies on base name from inheritance chain; inherits same base resolution fragility. | (4) |
| Attribute type names | Should resolve like any type. | Not audited; attribute resolver likely uses direct type lookup (needs confirmation). | (1)/(4) |
| Generic type names in inheritance (`Base<int>`) | Resolve base type normally; arity mismatch diagnosed. | Resolution of generic base depends on recorded name; if base type unresolved, downstream errors cascade. | (3)/(4) |
| Ambiguity handling | Single diagnostic listing candidates with qualify/alias help. | Current `type_is_subclass_of_name` accepts first match; ambiguity reporting not surfaced in base lists. | (1)/(4) |

## Concrete repro snippets (expected to fail today)

1) **Base via import only**
```
namespace A { public class Base {} }
namespace B { import A; public class Derived : Base {} }
```
Expected: resolves. Suspect: base lookup may fail without FQN.

2) **Ambiguous base across imports**
```
namespace X { public class Base {} }
namespace Y { public class Base {} }
namespace Z {
    import X;
    import Y;
    public class Derived : Base {} // should report ambiguity with both candidates
}
```
Expected: single ambiguity diagnostic. Current: may pick one arbitrarily or error “unknown type”.

3) **FQN vs unqualified stability**
```
namespace A { public class Base {} }
namespace A { public class Derived1 : Base {} public class Derived2 : A.Base {} }
```
Expected: both compile identically. Current: FQN path works; unqualified may depend on namespace capture/string.

4) **Package boundary visibility in bases**
Package A:
```
namespace P; internal class Hidden {}
namespace P; public class Exposed : Hidden {} // ok inside package
```
Package B (depends on A):
```
namespace Q; import P; public class Bad : Hidden {} // should be rejected with clear diagnostic
```
Current: because base lookup relies on namespace strings, may accept if namespaces match.

5) **Generic base resolution**
```
namespace A { public class G<T> {} }
namespace B { import A; public class Derived : G<int> {} }
```
Expected: resolves; missing type args should be a targeted diagnostic, not “unknown base”.

## Root-cause classification (likely)
- (4) Internal FQN string construction/bypass: base/interface validation uses stored `name` strings instead of running a resolver in context; ambiguity never surfaced.
- (2) Namespace merge/visibility: still using namespace equality for package visibility; may leak across packages.
- (1)/(3) Lookup order vs “type expected”: some paths may skip alias/import precedence when resolving inheritance targets.

## Fix plan (per bucket)
1. Add a canonical resolver entry point for type names in inheritance/interface lists that takes a `TypeExpr` + `ResolveContext` (current namespace, current type, imports, aliases, package visibility) and returns resolved symbol or ambiguity.
2. Remove/avoid string-based `resolve_type_info` calls for bases; ensure `BaseTypeBinding` stores the parsed `TypeExpr` and is resolved via the canonical path at typeck time.
3. Surface ambiguity diagnostic with candidates + qualify/alias help; use Rust-style spans.
4. Enforce visibility during base resolution (internal/package boundaries).
5. Add regression tests for: base via import, FQN vs unqualified, ambiguity across imports, generic bases, and cross-package internal base rejection.

## Tests to add (failing first)
- Unit typeck tests for base resolution (same namespace, via import, FQN).
- Ambiguity test with two imported `Base` types.
- Generic base resolution and missing type arguments.
- Cross-package internal base rejection (integration workspace).
- Regression ensuring no code path demands FQN by changing namespace but keeping imports.
