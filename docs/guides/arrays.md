# Array initialization

Chic mirrors the core C# array creation forms while keeping explicit, LL(1) grammar and deterministic lowering. Element types must always be named; no implicit `new[]` or collection-expression sugar is available.

## Supported forms
- `new T[n]` — allocates a length-`n` array and zero-initialises every slot before use.
- `new T[] { e1, e2, ... }` — infers the length from the initializer count.
- `new T[n] { e1, e2, ... }` — length must exactly match the initializer count (length expression must be a compile-time constant).
- Jagged arrays use arrays-of-arrays: `new int[][] { new int[] { 1 }, new int[] { 2, 3 } }`.

## Unsupported sugar (with diagnostics)
- Rectangular rank specifiers (`[,]`, `new T[2,3]`): rejected; use jagged `T[][]` instead.
- Implicitly typed literals (`new[] { ... }`), collection-expression literals (`[1,2,3]`), and brace-only initializers (`let xs: int[] = { 1, 2 };`): rejected; use `new T[] { ... }`.
- `new T[]` without a length or initializer is rejected; supply `new T[n]` or `new T[] { ... }`.

## Type and evaluation rules
- Element type `T` is explicit; every initializer expression must be convertible to `T`.
- Length expressions coerce to `usize`. When both a length and initializer are present, the length must be compile-time-constant and equal to the initializer count.
- Evaluation order: length expression (if present) is evaluated once, the array storage is zero-initialised, then initializer expressions are evaluated left-to-right and written in order. Length is set after initialization.
- Defaults: value types zero-initialise; nullable references become `null`; aggregates are defaulted before any initializer overlay. If an initializer fails, already-written slots remain valid and the array length does not advance past the last initialised element.

## Examples
```cl
var zeros = new int[3];                // [0, 0, 0]
var numbers = new int[] { 1, 2, 3 };   // length inferred as 3
var sized = new int[3] { 4, 5, 6 };    // length must match count
var jagged = new int[][] { new int[] { 7 }, new int[] { 8, 9 } };
```
