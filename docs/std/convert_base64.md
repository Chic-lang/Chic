# Std.Convert Base64

Chic’s Base64 helpers provide a familiar set of encoding/decoding APIs while keeping span-first, allocation-free paths for hot code. All output is ASCII-safe.

## APIs
- `Base64FormattingOptions { None, InsertLineBreaks }`
- Encoding: `Convert.ToBase64String(byte[])`, `Convert.ToBase64String(byte[], Base64FormattingOptions)`, `Convert.ToBase64String(byte[], int, int)`, `Convert.ToBase64String(byte[], int, int, Base64FormattingOptions)`, `Convert.ToBase64String(ReadOnlySpan<byte>, Base64FormattingOptions = None)`, `ToBase64CharArray(...)`, `TryToBase64Chars(ReadOnlySpan<byte>, Span<char>, out int, options = None)`
- Decoding: `Convert.FromBase64String(string)`, `FromBase64CharArray(char[], int, int)`, `TryFromBase64Chars(ReadOnlySpan<char>, Span<byte>, out int)`

## Behavior
- `InsertLineBreaks` inserts CRLF after every 76 encoded characters (no trailing break when the output is exactly 76).
- Decoding ignores ASCII whitespace (`' '`, `'\t'`, `'\r'`, `'\n'`).
- Padding rules: trimmed length must be a multiple of 4; `=` only appears in the final quartet (1 or 2 chars); any other placement fails with `FormatException` (or `false` for Try*).
- Deterministic across LLVM and WASM; same inputs always yield identical Base64 text.

## Examples
```cl
var data = new byte[] { 0xDEu8, 0xADu8, 0xBEu8, 0xEFu8 };
let encoded = Std.Convert.ToBase64String(ReadOnlySpan<byte>.FromArray(ref data));
// "3q2+7w=="

let header = Std.Convert.ToBase64String(
    "user:password".AsUtf8Span(),
    Base64FormattingOptions.None
);

let folded = Std.Convert.ToBase64String(
    ReadOnlySpan<byte>.FromArray(ref data),
    Base64FormattingOptions.InsertLineBreaks
); // CRLF every 76 chars

var buffer = new char[32];
if (Std.Convert.TryToBase64Chars(ReadOnlySpan<byte>.FromArray(ref data), Span<char>.FromArray(ref buffer), out var written))
{
    // buffer[0..written) contains the Base64 text without allocating a string
}
```

Decoding ignores whitespace:
```cl
let noisy = " AAE CAwQF\r\nBgcI\tCQoL DA0O Dw== ";
let bytes = Std.Convert.FromBase64String(noisy); // same as decoding the trimmed string
```

## Performance guidance
- Prefer `TryToBase64Chars` and `TryFromBase64Chars` to avoid intermediate strings when streaming HTTP headers, JSON blobs, or cryptography payloads.
- Use the encoded length formula (`((n + 2) / 3) * 4` plus CRLF pairs) to size buffers before calling Try* APIs; the methods return `false` and zero written when the destination is too small.
- Whitespace skipping during decode is branch-light and table-driven for predictable performance on both backends.

## Determinism
The encoder/decoder tables are fixed, padding and validation rules are explicit, and CRLF insertion is target-agnostic. LLVM and WASM produce identical Base64 output and accept the same inputs.
