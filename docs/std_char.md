# Std.Numeric.Char Reference

`Std.Numeric.Char` wraps Chic `char` values (UTF-16 code units) and implements the full numeric interface surface over an unsigned 16-bit integer. Classification and invariant casing helpers remain available for scalar-aware scenarios.

## Namespaces and Classes

- `Std.Numeric.Char`: primitive wrapper struct offering numeric operators plus classification/casing helpers.

## API Surface

| Member | Description |
| ------ | ----------- |
| `Char From(char value)` | Wraps the code unit into the `Char` value type. |
| `char ToChar()` | Extracts the underlying scalar. |
| `bool IsScalar(char value)` | Returns `true` when the value is a valid Unicode scalar (and not a surrogate). |
| `bool IsDigit(char value)` | Tests the scalar for numeric digit membership via `char::is_numeric`. |
| `bool IsLetter(char value)` | Tests the scalar for alphabetic membership. |
| `bool IsWhiteSpace(char value)` | Tests whether the scalar is whitespace. |
| `bool TryToUpperInvariant(char value, out Char result)` | Attempts to convert to uppercase using invariant casing. Returns `false` for invalid scalars or when the uppercase mapping expands to multiple scalars (e.g. `ß`); on failure the `result` remains the input value. |
| `Char ToUpperInvariant(Char value)` | Convenience wrapper that returns the converted scalar when the mapping is simple; otherwise returns the input unchanged. |
| `bool TryToLowerInvariant(char value, out Char result)` | Attempts to convert to lowercase using invariant casing with the same error semantics as `TryToUpperInvariant`. |
| `Char ToLowerInvariant(Char value)` | Convenience wrapper mirroring `ToUpperInvariant`. |
| `bool TryFromCodePoint(uint codePoint, out Char result)` | Validates and constructs a BMP code unit from a code point. Rejects surrogate ranges and code points above `0xFFFF`, leaving `result` as `'\0'` when returning `false`. |
| `string ToString(char value)` | Creates a single-code-unit string containing the supplied scalar. Invalid scalars map to the empty string. |

The struct also exposes `Zero`, `One`, `MinValue`, `MaxValue`, and implements the arithmetic/bitwise operator interfaces over the unsigned 16-bit domain.

## Error Semantics

The lower-level runtime functions return `CharError` values that map to the following meanings:

- `Success` (0): operation succeeded and the out parameters are populated.
- `InvalidScalar` (1): the supplied codepoint is not a Unicode scalar value.
- `NullPointer` (2): the runtime detected a null pointer in the out parameter (should not happen under managed usage).
- `ComplexMapping` (3): casing produced multiple scalars. The helper returns `false` in this scenario and leaves the result unchanged.

Runtime casing helpers (`chic_rt_char_to_upper` / `chic_rt_char_to_lower`) and `chic_rt_char_from_codepoint` encode their results into a packed 64-bit value. Callers must decode the `CharError` with `chic_rt_char_status(packed)` and, when the status indicates `Success` or `ComplexMapping`, extract the scalar via `chic_rt_char_value(packed)`. Complex mappings preserve the first scalar of the expansion so managed callers can surface a fallback without allocating temporary strings.

## Examples

```csharp
import Std.Numeric;

public bool IsEmojiDigit(char value)
{
    return Char.IsScalar(value) && Char.IsDigit(value);
}

public bool TryUppercase(char input, out char output)
{
    var success = Char.TryToUpperInvariant(input, out var result);
    output = result.ToChar();
    return success;
}

public string Render(uint codePoint)
{
    if (!Char.TryFromCodePoint(codePoint, out var scalar))
    {
        return string.Empty;
    }
    return scalar.ToString();
}
```

## Limitations

- Casing uses Rust's invariant mappings; culture-sensitive rules are not yet implemented.
- Multi-scalar expansions (e.g. `ß` → `SS`) report failure rather than performing string concatenation.
- The helpers operate synchronously; async-friendly iteration remains future work.
- Namespace imports (`import Std.Numeric;`) are not yet supported, so call sites must use fully qualified names (`Std.Numeric.Char.IsDigit` etc.).
