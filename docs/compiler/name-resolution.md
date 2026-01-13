# Name Resolution Guide

- **Lookup order (types):** aliases → current type/nested types → current namespace chain (inner → outer) → namespace imports → implicit `Std` import → fully-qualified path. Fully-qualified names (`Foo.Bar.Type`) always resolve directly.
- **When to import:** Prefer `import Foo;` when you routinely use multiple types from the same namespace.
- **When to alias:** Use `import Short = Long.Namespace.Type;` to disambiguate or to shorten a single frequently used type.
- **When to qualify:** Use `Foo.Bar.Type` at the call site when only one reference is ambiguous or when you need to show the defining package/module explicitly.
- Ambiguity is reported once with all candidates; qualify or alias to silence it. Missing bases/interfaces are errors—add the correct import or fix the name rather than relying on inferred qualification.
