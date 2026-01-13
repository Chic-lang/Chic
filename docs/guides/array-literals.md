# Array literals in Chic

Array literals let you write `[1, 2, 3]` without an explicit `new`. They follow the spec rules in `SPEC.md`:

- Default container: `T[]` with element type inferred from the elements.
- Prefix typing forces the container: `int[] [1, 2, 3]`, `Vec<string> ["a", "b"]` (Vec must be explicit).
- Conversions to `Span<T>`/`ReadOnlySpan<T>` borrow from the array allocated for the literal; the allocation is visible in MIR and follows normal lifetime/async rules.
- Empty literals require contextual element type: `[]` is an error unless the expected element type is known.

Examples:

```chic
// Inference to T[]
public int[] DefaultArray = [1, 2, 3];

// Explicit Vec<T>
public Vec<double> MakeVec() => Vec<double> [1.0, 2.5, 4.0];

// Borrow as span (allocation is explicit in MIR and the span borrows it)
public ReadOnlySpan<char> Letters() => ['a', 'b', 'c'];

// Nested literals
public int[][] Jagged = [[1], [2, 3]];
```

Diagnostics to expect:
- `[]` without a contextual type → “cannot infer element type for empty array literal” (`TYPE0705`).
- Heterogeneous elements that cannot unify emit a type mismatch diagnostic pointing at the conflicting indices.
- Brace-only forms (`{ 1, 2 }`) and rectangular ranks (`new int[2,3]`) remain rejected with targeted replacements.
