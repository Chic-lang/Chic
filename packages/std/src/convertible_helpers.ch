namespace Std;
import Std.Globalization;
import Std.Numeric;
import Std.Datetime;
import Std.Span;
import Std.Strings;
/// Shared conversion helpers used by primitive IConvertible implementations.
internal static class ConvertibleHelpers
{
    private const double INT128_MIN_DOUBLE = - 1.7014118346046923e38;
    private const double INT128_MAX_DOUBLE = 1.7014118346046923e38;
    private const double UINT128_MAX_DOUBLE = 3.4028236692093846e38;
    private const decimal INT64_MIN_DECIMAL = - 9223372036854775808m;
    private const decimal INT64_MAX_DECIMAL = 9223372036854775807m;
    private const decimal UINT64_MAX_DECIMAL = 18446744073709551615m;
    public static string ResolveCulture(IFormatProvider provider) => FormatProviderHelpers.ResolveCulture(provider);
    public static IDateTimeCulture ResolveDateTimeCulture(IFormatProvider provider) => DateTimeCultures.Resolve(ResolveCulture(provider));
    public static InvalidCastException InvalidConversion(string from, string to) => new InvalidCastException("Cannot convert " + from + " to " + to);
    public static OverflowException Overflow(string target) => new OverflowException(target + " was outside the valid range");
    public static Object ToTypeNotSupported(string fromType) => throw new NotSupportedException("IConvertible.ToType is not supported for " + fromType);
    private static decimal ParseInvariantDecimal(ref string text) {
        var utf8 = Std.Span.ReadOnlySpan.FromString(text);
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseDecimal (utf8, NumericUnchecked.ToByte ('.'), out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Decimal");
        }
        return parsed;
    }
    private static decimal ParseInvariantDecimal(ReadOnlySpan <byte >utf8) {
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseDecimal (utf8, NumericUnchecked.ToByte ('.'), out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Decimal");
        }
        return parsed;
    }
    private static long ParseInt64Invariant(ref string text) {
        var utf8 = Std.Span.ReadOnlySpan.FromString(text);
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseInt64 (utf8, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Int64");
        }
        return parsed;
    }
    private static ulong ParseUInt64Invariant(ref string text) {
        var utf8 = Std.Span.ReadOnlySpan.FromString(text);
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseUInt64 (utf8, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "UInt64");
        }
        return parsed;
    }
    private static int128 ParseInt128Invariant(ref string text) {
        var utf8 = Std.Span.ReadOnlySpan.FromString(text);
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseInt128 (utf8, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Int128");
        }
        return parsed;
    }
    private static u128 ParseUInt128Invariant(ref string text) {
        var utf8 = Std.Span.ReadOnlySpan.FromString(text);
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseUInt128 (utf8, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "UInt128");
        }
        return parsed;
    }
    public static decimal ToDecimalFromInt64(long value) {
        var text = NumericFormatting.FormatInt64(value, "G", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    public static decimal ToDecimalFromUInt64(ulong value) {
        var text = NumericFormatting.FormatUInt64(value, "G", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    public static decimal ToDecimalFromInt128(int128 value) {
        var text = NumericFormatting.FormatInt128(value, "G", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    public static decimal ToDecimalFromUInt128(u128 value) {
        var text = NumericFormatting.FormatUInt128(value, "G", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    public static decimal ToDecimalFromNInt(nint value) {
        var text = NumericFormatting.FormatIntPtr(value, "G", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    public static decimal ToDecimalFromNUInt(nuint value) {
        var text = NumericFormatting.FormatUIntPtr(value, "G", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    public static decimal ToDecimalFromFloat32(float value) {
        EnsureFinite((double) value, "Decimal");
        var text = NumericFormatting.FormatFloat32(value, "G9", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    public static decimal ToDecimalFromFloat64(double value) {
        EnsureFinite(value, "Decimal");
        var text = NumericFormatting.FormatFloat64(value, "G17", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    public static decimal ToDecimalFromFloat128(double value) {
        EnsureFinite(value, "Decimal");
        var text = NumericFormatting.FormatFloat64(value, "G17", "invariant");
        return ParseInvariantDecimal(ref text);
    }
    // Boolean helpers -----------------------------------------------------
    public static bool FromBoolean(bool value) => value;
    public static bool FromChar(char value) => value != '\0';
    public static bool FromInt64(long value) => value != 0;
    public static bool FromUInt64(ulong value) => value != 0ul;
    public static bool FromInt128(int128 value) => value != 0;
    public static bool FromUInt128(u128 value) => value != 0u128;
    public static bool FromDouble(double value) => value != 0.0d;
    public static bool FromDecimal(decimal value) => value != 0m;
    // Range-checked integral conversions ---------------------------------
    public static sbyte ToSByteChecked(long value) {
        if (value <SByte.MinValue || value >SByte.MaxValue)
        {
            throw new OverflowException("Value does not fit in SByte");
        }
        return NumericUnchecked.ToSByte(value);
    }
    public static byte ToByteChecked(long value) {
        if (value <Byte.MinValue || value >Byte.MaxValue)
        {
            throw new OverflowException("Value does not fit in Byte");
        }
        return NumericUnchecked.ToByte(value);
    }
    public static short ToInt16Checked(long value) {
        if (value <Int16.MinValue || value >Int16.MaxValue)
        {
            throw new OverflowException("Value does not fit in Int16");
        }
        return NumericUnchecked.ToInt16(value);
    }
    public static ushort ToUInt16Checked(long value) {
        if (value <UInt16.MinValue || value >UInt16.MaxValue)
        {
            throw new OverflowException("Value does not fit in UInt16");
        }
        return NumericUnchecked.ToUInt16(value);
    }
    public static int ToInt32Checked(long value) {
        if (value <Int32.MinValue || value >Int32.MaxValue)
        {
            throw new OverflowException("Value does not fit in Int32");
        }
        return NumericUnchecked.ToInt32(value);
    }
    public static uint ToUInt32Checked(long value) {
        if (value <UInt32.MinValue || value >UInt32.MaxValue)
        {
            throw new OverflowException("Value does not fit in UInt32");
        }
        return NumericUnchecked.ToUInt32(value);
    }
    public static long ToInt64Checked(long value) => value;
    public static ulong ToUInt64Checked(long value) {
        if (value <0)
        {
            throw new OverflowException("Value does not fit in UInt64");
        }
        return NumericUnchecked.ToUInt64(value);
    }
    public static sbyte ToSByteChecked(ulong value) {
        if (value >NumericUnchecked.ToUInt64 (SByte.MaxValue))
        {
            throw new OverflowException("Value does not fit in SByte");
        }
        return NumericUnchecked.ToSByte(NumericUnchecked.ToInt64(value));
    }
    public static byte ToByteChecked(ulong value) {
        if (value >NumericUnchecked.ToUInt64 (Byte.MaxValue))
        {
            throw new OverflowException("Value does not fit in Byte");
        }
        return NumericUnchecked.ToByte(NumericUnchecked.ToInt64(value));
    }
    public static short ToInt16Checked(ulong value) {
        if (value >NumericUnchecked.ToUInt64 (Int16.MaxValue))
        {
            throw new OverflowException("Value does not fit in Int16");
        }
        return NumericUnchecked.ToInt16(NumericUnchecked.ToInt64(value));
    }
    public static ushort ToUInt16Checked(ulong value) {
        if (value >NumericUnchecked.ToUInt64 (UInt16.MaxValue))
        {
            throw new OverflowException("Value does not fit in UInt16");
        }
        return NumericUnchecked.ToUInt16(NumericUnchecked.ToInt64(value));
    }
    public static int ToInt32Checked(ulong value) {
        if (value >NumericUnchecked.ToUInt64 (Int32.MaxValue))
        {
            throw new OverflowException("Value does not fit in Int32");
        }
        return NumericUnchecked.ToInt32(NumericUnchecked.ToInt64(value));
    }
    public static uint ToUInt32Checked(ulong value) {
        if (value >UInt32.MaxValue)
        {
            throw new OverflowException("Value does not fit in UInt32");
        }
        return NumericUnchecked.ToUInt32(value);
    }
    public static long ToInt64Checked(ulong value) {
        if (value >NumericUnchecked.ToUInt64 (Int64.MaxValue))
        {
            throw new OverflowException("Value does not fit in Int64");
        }
        return NumericUnchecked.ToInt64(value);
    }
    public static ulong ToUInt64Checked(ulong value) => value;
    public static char ToCharChecked(long value) {
        if (value <Char.MinValue || value >Char.MaxValue)
        {
            throw new OverflowException("Value does not fit in Char");
        }
        return NumericUnchecked.ToChar(value);
    }
    public static char ToCharChecked(ulong value) {
        if (value >NumericUnchecked.ToUInt64 (Char.MaxValue))
        {
            throw new OverflowException("Value does not fit in Char");
        }
        return NumericUnchecked.ToChar(NumericUnchecked.ToInt64(value));
    }
    public static char ToCharChecked(int128 value) {
        if (value <NumericUnchecked.ToInt64 (Char.MinValue) || value >NumericUnchecked.ToInt64 (Char.MaxValue))
        {
            throw new OverflowException("Value does not fit in Char");
        }
        return NumericUnchecked.ToChar((long) value);
    }
    public static char ToCharChecked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (NumericUnchecked.ToUInt64 (Char.MaxValue)))
        {
            throw new OverflowException("Value does not fit in Char");
        }
        return NumericUnchecked.ToChar(NumericUnchecked.ToInt64(NumericUnchecked.ToUInt64FromUInt128(value)));
    }
    // Pointer conversions --------------------------------------------------
    public static nint ToNIntChecked(long value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            if (value <NumericUnchecked.ToInt64 (NumericPlatform.IntPtrMinValue) || value >NumericUnchecked.ToInt64 (NumericPlatform.IntPtrMaxValue))
            {
                throw new OverflowException("Value does not fit in nint");
            }
            return NumericUnchecked.ToNintFromInt32(NumericUnchecked.ToInt32(value));
        }
        return(nint) value;
    }
    public static nint ToNIntChecked(ulong value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            if (value >NumericUnchecked.ToUInt64 (NumericPlatform.IntPtrMaxValue))
            {
                throw new OverflowException("Value does not fit in nint");
            }
            return NumericUnchecked.ToNintFromInt32(NumericUnchecked.ToInt32(NumericUnchecked.ToInt64(value)));
        }
        if (value >NumericUnchecked.ToUInt64 (NumericPlatform.IntPtrMaxValue))
        {
            throw new OverflowException("Value does not fit in nint");
        }
        return NumericUnchecked.ToNintFromInt64(NumericUnchecked.ToInt64(value));
    }
    public static nuint ToNUIntChecked(long value) {
        if (value <0)
        {
            throw new OverflowException("Value does not fit in nuint");
        }
        if (NumericPlatform.PointerBits == 32u)
        {
            if (value >NumericUnchecked.ToInt64 (NumericPlatform.UIntPtrMaxValue))
            {
                throw new OverflowException("Value does not fit in nuint");
            }
            return NumericUnchecked.ToNuintNarrow(NumericUnchecked.ToUInt32(value));
        }
        return NumericUnchecked.ToNuintWiden(NumericUnchecked.ToUInt64(value));
    }
    public static nuint ToNUIntChecked(ulong value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            if (value >NumericUnchecked.ToUInt64 (NumericPlatform.UIntPtrMaxValue))
            {
                throw new OverflowException("Value does not fit in nuint");
            }
            return NumericUnchecked.ToNuintNarrow(NumericUnchecked.ToUInt32(value));
        }
        return NumericUnchecked.ToNuintWiden(value);
    }
    public static isize ToISizeChecked(long value) => (isize) ToNIntChecked(value);
    public static isize ToISizeChecked(ulong value) => (isize) ToNIntChecked(value);
    public static usize ToUSizeChecked(long value) => (usize) ToNUIntChecked(value);
    public static usize ToUSizeChecked(ulong value) => (usize) ToNUIntChecked(value);
    // 128-bit integer conversions -----------------------------------------
    public static int128 ToInt128Checked(int128 value) => value;
    public static sbyte ToSByteChecked(int128 value) {
        if (value <SByte.MinValue || value >SByte.MaxValue)
        {
            throw new OverflowException("Value does not fit in SByte");
        }
        return(sbyte) value;
    }
    public static byte ToByteChecked(int128 value) {
        if (value <Byte.MinValue || value >Byte.MaxValue)
        {
            throw new OverflowException("Value does not fit in Byte");
        }
        return(byte) value;
    }
    public static short ToInt16Checked(int128 value) {
        if (value <Int16.MinValue || value >Int16.MaxValue)
        {
            throw new OverflowException("Value does not fit in Int16");
        }
        return(short) value;
    }
    public static ushort ToUInt16Checked(int128 value) {
        if (value <UInt16.MinValue || value >UInt16.MaxValue)
        {
            throw new OverflowException("Value does not fit in UInt16");
        }
        return(ushort) value;
    }
    public static int ToInt32Checked(int128 value) {
        if (value <Int32.MinValue || value >Int32.MaxValue)
        {
            throw new OverflowException("Value does not fit in Int32");
        }
        return(int) value;
    }
    public static uint ToUInt32Checked(int128 value) {
        if (value <UInt32.MinValue || value >UInt32.MaxValue)
        {
            throw new OverflowException("Value does not fit in UInt32");
        }
        return(uint) value;
    }
    public static long ToInt64Checked(int128 value) {
        if (value <Int64.MinValue || value >Int64.MaxValue)
        {
            throw new OverflowException("Value does not fit in Int64");
        }
        return(long) value;
    }
    public static ulong ToUInt64Checked(int128 value) {
        if (value <0 || value >NumericUnchecked.ToInt128 (UInt64.MaxValue))
        {
            throw new OverflowException("Value does not fit in UInt64");
        }
        return(ulong) value;
    }
    public static u128 ToUInt128Checked(int128 value) {
        if (value <0)
        {
            throw new OverflowException("Value does not fit in UInt128");
        }
        return NumericUnchecked.ToUInt128(value);
    }
    public static int128 ToInt128Checked(u128 value) {
        if (value >NumericConstants.Int128Max)
        {
            throw new OverflowException("Value does not fit in Int128");
        }
        return NumericUnchecked.ToInt128(value);
    }
    public static sbyte ToSByteChecked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (SByte.MaxValue))
        {
            throw new OverflowException("Value does not fit in SByte");
        }
        return(sbyte) NumericUnchecked.ToInt64(NumericUnchecked.ToUInt64FromUInt128(value));
    }
    public static byte ToByteChecked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (Byte.MaxValue))
        {
            throw new OverflowException("Value does not fit in Byte");
        }
        return(byte) NumericUnchecked.ToUInt64FromUInt128(value);
    }
    public static short ToInt16Checked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (Int16.MaxValue))
        {
            throw new OverflowException("Value does not fit in Int16");
        }
        return(short) NumericUnchecked.ToInt64(NumericUnchecked.ToUInt64FromUInt128(value));
    }
    public static ushort ToUInt16Checked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (UInt16.MaxValue))
        {
            throw new OverflowException("Value does not fit in UInt16");
        }
        return(ushort) NumericUnchecked.ToUInt64FromUInt128(value);
    }
    public static int ToInt32Checked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (Int32.MaxValue))
        {
            throw new OverflowException("Value does not fit in Int32");
        }
        return(int) NumericUnchecked.ToInt64(NumericUnchecked.ToUInt64FromUInt128(value));
    }
    public static uint ToUInt32Checked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (UInt32.MaxValue))
        {
            throw new OverflowException("Value does not fit in UInt32");
        }
        return(uint) NumericUnchecked.ToUInt64FromUInt128(value);
    }
    public static long ToInt64Checked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (Int64.MaxValue))
        {
            throw new OverflowException("Value does not fit in Int64");
        }
        return NumericUnchecked.ToInt64(NumericUnchecked.ToUInt64FromUInt128(value));
    }
    public static ulong ToUInt64Checked(u128 value) {
        if (value >NumericUnchecked.ToUInt128 (UInt64.MaxValue))
        {
            throw new OverflowException("Value does not fit in UInt64");
        }
        return NumericUnchecked.ToUInt64FromUInt128(value);
    }
    public static u128 ToUInt128Checked(u128 value) => value;
    public static int128 ToInt128Checked(long value) => (int128) value;
    public static u128 ToUInt128Checked(long value) {
        if (value <0)
        {
            throw new OverflowException("Value does not fit in UInt128");
        }
        return NumericUnchecked.ToUInt128(value);
    }
    public static int128 ToInt128Checked(ulong value) => NumericUnchecked.ToInt128(value);
    public static u128 ToUInt128Checked(ulong value) => NumericUnchecked.ToUInt128(value);
    // Floating conversions -------------------------------------------------
    private static bool IsNaN(double value) => value != value;
    private static bool IsInfinity(double value) => value == Float64.PositiveInfinity || value == Float64.NegativeInfinity;
    private static void EnsureFinite(double value, string target) {
        if (IsNaN (value) || IsInfinity (value))
        {
            throw new InvalidCastException("Cannot convert non-finite floating point value to " + target);
        }
    }
    public static sbyte ToSByteChecked(double value) {
        EnsureFinite(value, "SByte");
        if (value <SByte.MinValue || value >SByte.MaxValue)
        {
            throw new OverflowException("Value does not fit in SByte");
        }
        return NumericUnchecked.ToSByte((long) value);
    }
    public static byte ToByteChecked(double value) {
        EnsureFinite(value, "Byte");
        if (value <Byte.MinValue || value >Byte.MaxValue)
        {
            throw new OverflowException("Value does not fit in Byte");
        }
        return NumericUnchecked.ToByte((long) value);
    }
    public static short ToInt16Checked(double value) {
        EnsureFinite(value, "Int16");
        if (value <Int16.MinValue || value >Int16.MaxValue)
        {
            throw new OverflowException("Value does not fit in Int16");
        }
        return NumericUnchecked.ToInt16((long) value);
    }
    public static ushort ToUInt16Checked(double value) {
        EnsureFinite(value, "UInt16");
        if (value <UInt16.MinValue || value >UInt16.MaxValue)
        {
            throw new OverflowException("Value does not fit in UInt16");
        }
        return NumericUnchecked.ToUInt16((long) value);
    }
    public static int ToInt32Checked(double value) {
        EnsureFinite(value, "Int32");
        if (value <Int32.MinValue || value >Int32.MaxValue)
        {
            throw new OverflowException("Value does not fit in Int32");
        }
        return(int) value;
    }
    public static uint ToUInt32Checked(double value) {
        EnsureFinite(value, "UInt32");
        if (value <UInt32.MinValue || value >UInt32.MaxValue)
        {
            throw new OverflowException("Value does not fit in UInt32");
        }
        return(uint) value;
    }
    public static long ToInt64Checked(double value) {
        EnsureFinite(value, "Int64");
        if (value <Int64.MinValue || value >Int64.MaxValue)
        {
            throw new OverflowException("Value does not fit in Int64");
        }
        return(long) value;
    }
    public static ulong ToUInt64Checked(double value) {
        EnsureFinite(value, "UInt64");
        if (value <UInt64.MinValue || value >UInt64.MaxValue)
        {
            throw new OverflowException("Value does not fit in UInt64");
        }
        return(ulong) value;
    }
    public static int128 ToInt128Checked(double value) {
        EnsureFinite(value, "Int128");
        if (value <INT128_MIN_DOUBLE || value >INT128_MAX_DOUBLE)
        {
            throw new OverflowException("Value does not fit in Int128");
        }
        return(int128) value;
    }
    public static u128 ToUInt128Checked(double value) {
        EnsureFinite(value, "UInt128");
        if (value <0.0d || value >UINT128_MAX_DOUBLE)
        {
            throw new OverflowException("Value does not fit in UInt128");
        }
        return(u128) value;
    }
    public static char ToCharChecked(double value) {
        EnsureFinite(value, "Char");
        if (value <NumericUnchecked.ToInt64 (Char.MinValue) || value >NumericUnchecked.ToInt64 (Char.MaxValue))
        {
            throw new OverflowException("Value does not fit in Char");
        }
        return(char)(int) value;
    }
    // Decimal helpers -----------------------------------------------------
    private static decimal TruncateDecimal(decimal value) {
        if (value >= 0m)
        {
            return value - (value % 1m);
        }
        return value + ((- value) % 1m) * - 1m;
    }
    private static bool HasFraction(decimal value) {
        return value % 1m != 0m;
    }
    public static double ToFloat64FromDecimal(decimal value) {
        var text = NumericFormatting.FormatDecimal(value, "G", "invariant");
        var utf8 = Std.Span.ReadOnlySpan.FromString(text);
        var index = 0usize;
        var negative = false;
        if (utf8.Length >0 && utf8[0] == NumericUnchecked.ToByte ('-'))
        {
            negative = true;
            index = 1usize;
        }
        var acc = 0.0d;
        var fracDigits = 0;
        var seenDecimal = false;
        while (index <utf8.Length)
        {
            let ch = utf8[index];
            if (ch == NumericUnchecked.ToByte ('.'))
            {
                if (seenDecimal)
                {
                    throw new Std.FormatException("Decimal text was not in a recognised format");
                }
                seenDecimal = true;
                index += 1;
                continue;
            }
            let digit = ch - NumericUnchecked.ToByte('0');
            acc = (acc * 10.0d) + NumericUnchecked.ToFloat64(NumericUnchecked.ToUInt32(digit));
            if (seenDecimal)
            {
                fracDigits += 1;
            }
            index += 1;
        }
        while (fracDigits >0)
        {
            acc = acc / 10.0d;
            fracDigits -= 1;
        }
        return negative ?- acc : acc;
    }
    public static float ToFloat32FromDecimal(decimal value) => (float) ToFloat64FromDecimal(value);
    public static long ToInt64Checked(decimal value) {
        if (value <INT64_MIN_DECIMAL || value >INT64_MAX_DECIMAL)
        {
            throw new OverflowException("Value does not fit in Int64");
        }
        if (HasFraction (value))
        {
            throw new OverflowException("Fractional Decimal cannot be converted to Int64");
        }
        var text = NumericFormatting.FormatDecimal(value, "F0", "invariant");
        return ParseInt64Invariant(ref text);
    }
    public static ulong ToUInt64Checked(decimal value) {
        if (value <0m || value >UINT64_MAX_DECIMAL)
        {
            throw new OverflowException("Value does not fit in UInt64");
        }
        if (HasFraction (value))
        {
            throw new OverflowException("Fractional Decimal cannot be converted to UInt64");
        }
        var text = NumericFormatting.FormatDecimal(value, "F0", "invariant");
        return ParseUInt64Invariant(ref text);
    }
    public static int128 ToInt128Checked(decimal value) {
        if (HasFraction (value))
        {
            throw new OverflowException("Fractional Decimal cannot be converted to Int128");
        }
        var text = NumericFormatting.FormatDecimal(value, "F0", "invariant");
        return ParseInt128Invariant(ref text);
    }
    public static u128 ToUInt128Checked(decimal value) {
        if (value <0m || HasFraction (value))
        {
            throw new OverflowException("Decimal cannot be converted to UInt128");
        }
        var text = NumericFormatting.FormatDecimal(value, "F0", "invariant");
        return ParseUInt128Invariant(ref text);
    }
    public static Object ConvertFromFloat32(double value, Type targetType, string culture, string fromType) {
        return ToTypeNotSupported(fromType);
    }
    public static Object ConvertFromFloat64(double value, Type targetType, string culture, string fromType) {
        return ToTypeNotSupported(fromType);
    }
    // Type dispatchers ----------------------------------------------------
    public static Object ConvertFromInt64(long value, Type targetType, string culture, string fromType) {
        return ToTypeNotSupported(fromType);
    }
    public static Object ConvertFromUInt64(ulong value, Type targetType, string culture, string fromType) {
        return ToTypeNotSupported(fromType);
    }
    public static Object ConvertFromInt128(int128 value, Type targetType, string culture, string fromType) {
        return ToTypeNotSupported(fromType);
    }
    public static Object ConvertFromUInt128(u128 value, Type targetType, string culture, string fromType) {
        return ToTypeNotSupported(fromType);
    }
}
