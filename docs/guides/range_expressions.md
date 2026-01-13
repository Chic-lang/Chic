# Range expressions and slicing

Chic now understands C#-style range/index-from-end syntax across the core collection
types. A range literal never allocates—it materialises value types from `Std.Range` that
carry the start/end offsets.

- **Syntax**: `start..end` (exclusive), `start..=` (inclusive), `start..`, `..end`, `..`,
  and index-from-end using `^expr`. Chained operators (e.g., `a..b..c`) are rejected.
- **Containers**: `Span<T>`, `ReadOnlySpan<T>`, `Vec<T>`, `Array<T>`, and `string/str`
  expose `Slice` overloads for all range shapes. Strings surface UTF-8 views via
  `ReadOnlySpan<byte>`.
- **Bounds**: `^0` is invalid; inclusive ends require `end < len`; open-ended ranges
  compute bounds against the container length and panic with the corresponding `SpanError`
  codes on failure.
- **Iteration**: `foreach` over `Range`/`RangeInclusive` lowers to a counted loop without
  allocating an enumerator. Open-ended ranges (`..end`, `start..`, `..`) and from-end
  endpoints are rejected because there is no ambient length to resolve them against.

### Examples

```chic
import Std.Range;
import Std.Span;

public Span<int> Middle(Span<int> values) => values[1..^1];
public ReadOnlySpan<byte> Header(string line) => line[..4];

public int SumRange(int start, int end)
{
    var total = 0;
    foreach (var i in start..end)
    {
        total += i;
    }
    return total;
}
```

See the language specification section **Range Expressions** for full precedence and
evaluation rules.
