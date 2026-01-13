# Primitive byte conversions

`Std.BitConverter` provides deterministic, span-first conversions between primitive values and raw bytes. All APIs are explicit about endianness and avoid allocations unless you call the `GetBytes` helpers.

- `Endianness` – `Little` or `Big`, plus `BitConverter.NativeEndianness` for discovery only.
- Span-first helpers – `TryWrite*`/`TryRead*` for integers (sbyte through uint128, nint, nuint, usize), floats (`float16`, `float`, `double`, `Float128`), `decimal`, `Uuid`, `bool`, and `char`.
- `GetBytes` – convenience overloads that allocate exactly the required length and forward to the span-based writers (default endianness: little).
- Bulk helpers – `ReverseEndiannessInPlace` for 2/4/8/16 byte spans and scalar `ReverseEndianness` for `ushort`/`uint`/`ulong`.

Rules and layouts:

- Integers are emitted in two's-complement form; endianness drives byte ordering only.
- Floats use the raw IEEE-754 bit pattern; NaN payloads and signed zero are preserved. No normalisation or host-endian assumptions.
- `bool` encodes as a single byte (`0` or `1`); any other value fails `TryReadBoolean`.
- `char` encodes as a 16-bit UTF-16 code unit.
- `decimal` uses the declared `Decimal128Parts` layout: `{ lo:u32, mid:u32, hi:u32, flags:u32 }`. Field order is fixed; endianness applies to each 32-bit component. The `flags` payload encodes scale in bits 16–23 and sign in the high bit.
- `Uuid` uses RFC 4122 network ordering when `Endianness.Big`; `Endianness.Little` reverses those 16 bytes. No mixed-field ordering is used.

## Examples

Write an `int` to a stack-allocated buffer (little endian):

```cl
Span<byte> scratch = Span<byte>.StackAlloc(4);
BitConverter.TryWriteInt32(scratch, 0x01020304, Endianness.Little, out var written);
// scratch now contains 04-03-02-01
```

Parse a big-endian value from a network packet:

```cl
ReadOnlySpan<byte> packet = Span<byte>.FromArray(ref payload).AsReadOnly();
if (!BitConverter.TryReadUInt32(packet.Slice(0usize, 4usize), Endianness.Big, out var remote, out _))
{
    throw new Std.InvalidOperationException("packet too small");
}
```

Serialize a `Uuid` and `decimal` deterministically:

```cl
let id = Uuid.Parse("00112233-4455-6677-8899-aabbccddeeff");
var uuidBytes = BitConverter.GetBytes(id, Endianness.Big); // RFC 4122 order

let balance = 12345.625m;
var balanceBytes = BitConverter.GetBytes(balance, Endianness.Little); // lo/mid/hi/flags (LE per field)
```
