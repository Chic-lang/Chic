# Working with Numeric Literals

This guide describes Chic numeric literal syntax and demonstrates common patterns.

## Prefixes & Separators

| Form | Example | Notes |
|------|---------|-------|
| Decimal | `1_000_000` | Separators may appear between digits only. |
| Binary | `0b_1010_0110u8` | Separators may follow the prefix. |
| Hexadecimal | `0x_FF_A0i32` | Case-insensitive prefix. |

Invalid placements (rejected during lexing):

```cl
let bad = 1._0;      // separator adjacent to decimal point
let also_bad = _512; // leading separator
let nope = 0xFF_;    // trailing separator
```

## Suffix Matrix

| Category | Suffixes | Resulting Type |
|----------|----------|----------------|
| Signed integers | `i8`, `i16`, `i32`, `i64`, `i128`, `isize` | `sbyte`, `short`, `int`, `long`, `int128`, `nint` |
| Unsigned integers | `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `u` | `byte`, `ushort`, `uint`, `ulong`, `uint128`, `nuint`, `uint` |
| Floating point | `f`, `f32`, `d`, `D`, `double`, `f64` | `float`, `double` |
| Decimal | `m`, `M` | `decimal` |

Some suffixes have multiple accepted spellings; prefer the explicit width suffixes (`i32`, `u64`, etc.) in new code.

## Type Inference & Diagnostics

* Unsuffixed integers adopt the contextual type (`int` if no context is available, `uint` when assigned to unsigned storage). Unsuffixed reals default to `double`.
* `[TCK120]` is emitted when the literal suffix disagrees with the destination type.
* `[TCK121]` reports when a literal’s value exceeds the suffix’s range (`300u8`, `0x1_0000_0000u32`, etc.).

```cl
let workers: ushort = 100u16;       // ok
let bigger: ushort = 70000u16;      // [TCK121]
let promoted: long = 42u8;          // [TCK120] -> use 42 or 42i64
let precise = 1.0e-3f32;            // single precision
let money   = 12_345.67m;           // decimal literal
```

## Backend Considerations

* LLVM observes literal metadata directly; the emitted IR uses the matching integer/float width.
* The WASM backend currently supports integral literals up to 64 bits and emits demote/promote instructions for mismatched float widths. Attempting to lower `u128` results in a diagnostic—split the value or use helper constructors instead.

## Helper APIs

Once literals land in Chic structs you can lean on the new helper surface for overflow checks, bit fiddling, and formatting:

```cl
let ok = Int32.TryAdd(Int32.From(2_000_000_000), Int32.From(147_483_647), out var sum);
assert(ok && sum.ToInt32() == 2_147_483_647);
assert(!Int32.TryAdd(Int32.From(Int32.MaxValue), Int32.From(1), out sum));

assert(Int32.PopCount(Int32.From(-1)) == 32);
assert(Int32.LeadingZeroCount(Int32.From(1)) == 31);
assert(Int32.RotateLeft(Int32.From(1), 5).ToInt32() == 32);
assert(Int32.ReverseEndianness(Int32.From(0x01020304)).ToInt32() == 0x04030201);
assert(Int32.IsPowerOfTwo(Int32.From(1024)));

let hex = Int32.From(-1).ToString("X");   // "FFFFFFFF"
let bin = UInt32.From(255u).ToString("b"); // "11111111"

let ptr = IntPtr.RotateLeft(IntPtr.From((nint)1), 1);
assert(ptr.ToIntPtr() == (nint)2);
```

Pointer-sized structs delegate to the appropriate 32- or 64-bit implementation based on `NumericInfo.PointerBitWidth`, so `IntPtr.TryAdd` and `UIntPtr.RotateLeft` stay consistent across architectures.

## Additional Resources

- Specification: `SPEC.md#numeric-literal-syntax`
