# Std.Uuid

`Std.Uuid` is a 128-bit value type that follows the RFC 4122 wire layout. Generation uses UUID v4 (random) with a cryptographically secure RNG, and all parsing/formatting routines are deterministic across LLVM/WASM backends.

## Endianness and byte layout

- `WriteBytes`/`TryWriteBytes` and the `byte[]`/`ReadOnlySpan<byte>` constructors use RFC 4122 network order: the first byte in the span is the first byte in the canonical string (`00112233-4455-...` -> `00 11 22 33 44 55 ...`).
- There is no mixed-endian layout; all byte APIs are strictly network order.

## String formats

`ToString` defaults to `"D"`. The following format specifiers are accepted (case-insensitive):

- `D`: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
- `N`: `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
- `B`: `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`
- `P`: `(xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)`

Parsing rejects whitespace and unsupported `"X"` format specifiers.

## Examples

```chic
let id = Uuid.NewUuid();
let text = id.ToString(); // default "D" format

let parsed = Uuid.Parse("00112233-4455-6677-8899-aabbccddeeff");
if (Uuid.TryParse("bad-value", out var maybe))
{
    // never reached for invalid input
}

var bytes = Span<byte>.StackAlloc(16);
parsed.WriteBytes(bytes); // RFC 4122 order
```
