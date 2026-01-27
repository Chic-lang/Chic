namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "ulong", kind = "int", bits = 64, signed = false,
aliases = ["ulong", "UInt64", "Std.UInt64", "Std.Numeric.UInt64", "System.UInt64", "uint64", "u64"], c_type = "uint64_t") public readonly struct UInt64 : IComparable, IComparable <ulong >, IConvertible, IEquatable <ulong >, IParsable <ulong >, ISpanParsable <ulong >, IUtf8SpanParsable <ulong >, IAdditionOperators <ulong, ulong, ulong >, IAdditiveIdentity <ulong, ulong >, IBinaryInteger <ulong >, IBinaryNumber <ulong >, IBitwiseOperators <ulong, ulong, ulong >, IComparisonOperators <ulong, ulong, bool >, IDecrementOperators <ulong >, IDivisionOperators <ulong, ulong, ulong >, IEqualityOperators <ulong, ulong, bool >, IIncrementOperators <ulong >, IMinMaxValue <ulong >, IModulusOperators <ulong, ulong, ulong >, IMultiplicativeIdentity <ulong, ulong >, IMultiplyOperators <ulong, ulong, ulong >, INumber <ulong >, INumberBase <ulong >, IShiftOperators <ulong, int, ulong >, ISubtractionOperators <ulong, ulong, ulong >, IUnaryPlusOperators <ulong, ulong >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly ulong value;
    public const ulong MinValue = 0ul;
    public const ulong MaxValue = 0xFFFF_FFFF_FFFF_FFFFul;
    public init(ulong value) {
        this.value = value;
    }
    public ulong ToUInt64() => value;
    public static UInt64 From(ulong value) => new UInt64(value);
    public static UInt64 Zero => new UInt64(0ul);
    public static UInt64 One => new UInt64(1ul);
    public static UInt64 AdditiveIdentity() => Zero;
    public static UInt64 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatUInt64(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromUInt64(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(value);
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(value);
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(value);
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(value);
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(value);
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(value);
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked(value);
    public long ToInt64(IFormatProvider provider) => ConvertibleHelpers.ToInt64Checked(value);
    public ulong ToUInt64(IFormatProvider provider) => value;
    public nint ToNInt(IFormatProvider provider) => ConvertibleHelpers.ToNIntChecked(value);
    public nuint ToNUInt(IFormatProvider provider) => ConvertibleHelpers.ToNUIntChecked(value);
    public isize ToISize(IFormatProvider provider) => ConvertibleHelpers.ToISizeChecked(value);
    public usize ToUSize(IFormatProvider provider) => ConvertibleHelpers.ToUSizeChecked(value);
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(value);
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(value);
    public float ToSingle(IFormatProvider provider) => (float) value;
    public double ToDouble(IFormatProvider provider) => (double) value;
    public Float128 ToFloat128(IFormatProvider provider) => new Float128((double) value);
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromUInt64(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("UInt64",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("UInt64");
    public string Format(string format, string culture) => NumericFormatting.FormatUInt64(value, format, culture);
    public static UInt64 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out UInt64 result) {
        var parsed = 0ul;
        if (! NumericParse.TryParseUInt64 (text, out parsed)) {
            result = new UInt64(0ul);
            return false;
        }
        result = new UInt64(parsed);
        return true;
    }
    public static UInt64 Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (! NumericParse.TryParseUInt64 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "UInt64");
        }
        return new UInt64(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out UInt64 result) {
        var parsed = 0ul;
        if (! NumericParse.TryParseUInt64 (text, out parsed)) {
            result = new UInt64(0ul);
            return false;
        }
        result = new UInt64(parsed);
        return true;
    }
    public int CompareTo(UInt64 other) {
        if (value <other.value)
        {
            return - 1;
        }
        if (value >other.value)
        {
            return 1;
        }
        return 0;
    }
    public int CompareTo(Object other) {
        throw new Std.ArgumentException("object is not a UInt64");
    }
    public static int Compare(UInt64 left, UInt64 right) {
        return left.CompareTo(right);
    }
    public static bool operator == (UInt64 left, UInt64 right) => left.value == right.value;
    public static bool operator != (UInt64 left, UInt64 right) => left.value != right.value;
    public static bool operator <(UInt64 left, UInt64 right) => left.value <right.value;
    public static bool operator <= (UInt64 left, UInt64 right) => left.value <= right.value;
    public static bool operator >(UInt64 left, UInt64 right) => left.value >right.value;
    public static bool operator >= (UInt64 left, UInt64 right) => left.value >= right.value;
    public static UInt64 operator + (UInt64 left, UInt64 right) => new UInt64(left.value + right.value);
    public static UInt64 operator - (UInt64 left, UInt64 right) => new UInt64(left.value - right.value);
    public static UInt64 operator * (UInt64 left, UInt64 right) => new UInt64(left.value * right.value);
    public static UInt64 operator / (UInt64 left, UInt64 right) => new UInt64(left.value / right.value);
    public static UInt64 operator % (UInt64 left, UInt64 right) => new UInt64(left.value % right.value);
    public static UInt64 operator + (UInt64 value) => value;
    public static UInt64 operator ++ (UInt64 value) => new UInt64(value.value + 1ul);
    public static UInt64 operator -- (UInt64 value) => new UInt64(value.value - 1ul);
    public static UInt64 operator & (UInt64 left, UInt64 right) => new UInt64(left.value & right.value);
    public static UInt64 operator | (UInt64 left, UInt64 right) => new UInt64(left.value | right.value);
    public static UInt64 operator ^ (UInt64 left, UInt64 right) => new UInt64(left.value ^ right.value);
    public static UInt64 operator ~ (UInt64 value) => new UInt64(value.value ^ 0xFFFFFFFFFFFFFFFFul);
    public static UInt64 operator << (UInt64 value, int offset) => new UInt64(value.value << offset);
    public static UInt64 operator >> (UInt64 value, int offset) => new UInt64(value.value >> offset);
    public static bool Equals(UInt64 left, UInt64 right) => left.value == right.value;
    public bool Equals(UInt64 other) => value == other.value;
    public static UInt64 Min(UInt64 left, UInt64 right) => left.value <= right.value ?left : right;
    public static UInt64 Max(UInt64 left, UInt64 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(UInt64 left, UInt64 right, out UInt64 result) {
        var raw = 0ul;
        if (! NumericArithmetic.TryAddUInt64 (left.value, right.value, out raw)) {
            result = new UInt64(0ul);
            return false;
        }
        result = new UInt64(raw);
        return true;
    }
    public static bool TrySubtract(UInt64 left, UInt64 right, out UInt64 result) {
        var raw = 0ul;
        if (! NumericArithmetic.TrySubtractUInt64 (left.value, right.value, out raw)) {
            result = new UInt64(0ul);
            return false;
        }
        result = new UInt64(raw);
        return true;
    }
    public static bool TryMultiply(UInt64 left, UInt64 right, out UInt64 result) {
        var raw = 0ul;
        if (! NumericArithmetic.TryMultiplyUInt64 (left.value, right.value, out raw)) {
            result = new UInt64(0ul);
            return false;
        }
        result = new UInt64(raw);
        return true;
    }
    public static int LeadingZeroCount(UInt64 value) {
        return NumericBitOperations.LeadingZeroCountUInt64(value.value);
    }
    public static int TrailingZeroCount(UInt64 value) {
        return NumericBitOperations.TrailingZeroCountUInt64(value.value);
    }
    public static int PopCount(UInt64 value) {
        return NumericBitOperations.PopCountUInt64(value.value);
    }
    public static UInt64 RotateLeft(UInt64 value, int offset) {
        return new UInt64(NumericBitOperations.RotateLeftUInt64(value.value, offset));
    }
    public static UInt64 RotateRight(UInt64 value, int offset) {
        return new UInt64(NumericBitOperations.RotateRightUInt64(value.value, offset));
    }
    public static UInt64 ReverseEndianness(UInt64 value) {
        return new UInt64(NumericBitOperations.ReverseEndiannessUInt64(value.value));
    }
    public static bool IsPowerOfTwo(UInt64 value) {
        return NumericBitOperations.IsPowerOfTwoUInt64(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatUInt64(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatUInt64(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatUInt64(value, destination, out written, format, culture);
    }
}
