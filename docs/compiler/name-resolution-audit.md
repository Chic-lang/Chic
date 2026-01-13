# Name Resolution Stability Audit

This audit captures the current gaps in Chic’s type/name resolution, the repros we can already trigger, and a fix+test plan. The focus is C#-like behavior for qualified/unqualified names, especially in base/interface lists and related lookups.

## Current vs expected

| Context | Expected (C#-like) | Current behavior | Root cause |
| --- | --- | --- | --- |
| Class/struct base list resolution | Unqualified bases resolve via aliases/current namespace/imports/Std prelude; fully-qualified always works; ambiguity emits a precise diagnostic. | Unqualified bases often fail or require manual FQNs when the base lives outside the immediate namespace (e.g., imported namespace). Resolution falls back to string-built `namespace::Name` or fails outright. | Internal bypass of canonical resolver in `module_lowering::resolve_type_for_expr` when `ty.base` is empty (skips import resolver and only matches literal visibility keys), plus `typeck::register_class` fallback to `qualify(namespace, base.name)` when not found. |
| Interface list resolution | Same as base list: unqualified/qualified both resolve with normal rules, ambiguity reported once. | Shares the same failure modes as base lists because interface resolution uses the same resolver paths; imported interfaces may require FQN or be silently qualified without disambiguation. | Same bypass + fallback string-building as above. |
| Struct/union layout bases (inheritance/traits) | Resolves with canonical rules; visibility enforced; ambiguity reported with candidates. | Uses the same non-canonical path; unqualified bases can be missed unless fully-qualified. | Same bypass path in `module_lowering::resolve_type_for_expr`. |
| Super/constructor chaining target resolution | Should reuse canonical type resolver so `super(...)` works with imported/unqualified bases. | Relies on the same resolver that bypasses imports for bare identifiers, so super targets can go unresolved unless fully-qualified. | Same bypass. |
| Ambiguity diagnostics for bases/interfaces | One error with primary span on the identifier, notes listing candidates + help to qualify/alias. | Current `AMBIGUOUS_CLASS_BASE`/interface errors are message-only (`ambiguous base type ... candidates: ...`) with no spans/notes/help; no package provenance shown. | Diagnostic path doesn’t attach spans/labels; no candidate metadata. |

## Bug list with repros

1) **Unqualified base fails across namespaces/imports**  
   - **Repro:**  
     ```chic
     namespace A; public class Base {}
     namespace B; import A; public class Derived : Base {}
     ```  
     Expected: compiles. Current: base resolution fails unless written `A.Base` (or equivalent FQN), because `module_lowering::resolve_type_for_expr` short-circuits when `ty.base` is empty and doesn’t consult imports/namespace chain.  
   - **Category:** Resolver lookup order bug + Internal bypass code.

2) **Interface resolution same failure**  
   - **Repro:**  
     ```chic
     namespace A; public interface IFoo {}
     namespace B; import A; public class Impl : IFoo {}
     ```  
     Expected: compiles. Current: requires `A.IFoo` (or fails) for the same reason as (1).  
   - **Category:** Resolver lookup order bug + Internal bypass code.

3) **Ambiguity lacks spans/help**  
   - **Repro:** Two imports exposing the same `Base`:
     ```chic
     import A; import B;
     namespace Test; public class Derived : Base {}
     ```  
     Expected: one error with primary label on `Base`, notes listing `A.Base`, `B.Base`, help to qualify/alias. Current: message-only `ambiguous base type Base; candidates: A.Base, B.Base` (no spans/help, unstable UX).  
   - **Category:** Diagnostic quality.

4) **String-built fallback hides missing/incorrect bases**  
   - **Repro:**  
     ```chic
     namespace Outer;
     public class Derived : MissingBase {}
     ```  
     Expected: clear “type not found” on `MissingBase`. Current: `typeck::register_class` falls back to `qualify(namespace, base.name)` on NotFound, recording a base binding to `Outer::MissingBase` even though it never resolved, which can leak through later stages.  
   - **Category:** Internal bypass code + Incorrect “type expected” resolution path.

5) **Super/constructor chaining resolution inherits the same bypass**  
   - **Repro:**  
     ```chic
     namespace A; public class Base { public init() {} }
     namespace B; import A; public class Derived : Base { public init() : super() {} }
     ```  
     Expected: `super()` resolves via imports. Current: can fail unless `Base` is written fully qualified because constructor target resolution uses the same `resolve_type_for_expr` shortcut.  
   - **Category:** Resolver lookup order bug + Internal bypass code.

## Fix plan

1) **Centralize resolution**  
   - Introduce/use a canonical resolver that always routes through `ImportResolver` (aliases, current type, namespace chain, imports, Std prelude) for all `TypeExpr` resolution, including cases where `ty.base` is empty. Remove the early-return shortcut in `module_lowering::resolve_type_for_expr`.
   - Remove fallback string-building (`qualify(namespace, base.name)`) in `typeck::register_class`/interface/struct registrations; require successful resolution or emit a proper diagnostic.

2) **Lookup order/ambiguity**  
   - Enforce the lookup order: aliases → current type/nesting → namespace chain (inner → outer) → imports → prelude → global/qualified.  
   - On multiple hits at the same tier, emit one ambiguity diagnostic with candidate list + spans/notes.

3) **Shared resolver for all contexts**  
   - Wire base/interface lists, constructor `super` targets, trait/interface refs, and any remaining call sites (layout resolution, module lowering) to the canonical resolver.
   - Add visibility checks after resolution (internal, protected, etc.).

4) **Diagnostics**  
   - Add Rust-style diagnostic with primary label on the unresolved/ambiguous identifier, secondary labels per candidate (with fully-qualified name + package), and help suggesting qualification or alias.

## Test plan

- **Compile-pass:**  
  1) Same-namespace base: `namespace A; class Base {}` + `class Derived : Base {}`.  
  2) Import-resolved base/interface: base in `A`, derived in `B` with `import A`.  
  3) Fully-qualified base always works (`class Derived : A.Base {}`).  
  4) Generic base: `class Base<T> {}` + `class Derived : Base<int> {}`.

- **Compile-fail:**  
  5) Ambiguous base via two imports: expect single diagnostic with candidates + help.  
  6) Missing base: clear “not found” on the identifier (no silent qualification).  
  7) Visibility: internal base in another package rejected with correct code/message.  
  8) Super-call resolution uses imports (deriving package B importing base package A).

- **Internal regression:**  
  - Test that module lowering resolves unqualified `Base` via imports (no requirement for pre-baked FQN strings).  
  - Verify no string concatenation fallback is used (e.g., failing resolution doesn’t produce a fabricated `namespace::Name` binding).  
  - Ambiguity diagnostic snapshot with labels/notes/help.
